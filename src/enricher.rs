//! Weather enrichment: (location, timestamp) -> real historical weather.
//!
//! Geocodes the location via [`crate::geocoder::Geocoder`], then fetches
//! real historical weather from Open-Meteo's free, no-API-key Historical
//! Weather (archive) API — genuinely observed temperature/humidity/weather
//! code for that place and hour, not a formula-generated placeholder.

use crate::geocoder::Geocoder;
use crate::types::EnrichedData;
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

const ARCHIVE_API_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

#[derive(Debug, Deserialize)]
struct ArchiveResponse {
    hourly: HourlyData,
}

#[derive(Debug, Deserialize)]
struct HourlyData {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    relative_humidity_2m: Vec<Option<f64>>,
    weather_code: Vec<Option<i64>>,
}

/// WMO weather interpretation codes (the standard Open-Meteo uses) mapped
/// to a human-readable condition string.
fn condition_from_wmo_code(code: i64) -> &'static str {
    match code {
        0 => "Clear",
        1 => "Mainly Clear",
        2 => "Partly Cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing Drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing Rain",
        71 | 73 | 75 => "Snow",
        77 => "Snow Grains",
        80 | 81 | 82 => "Rain Showers",
        85 | 86 => "Snow Showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with Hail",
        _ => "Unknown",
    }
}

fn parse_timestamp(timestamp: &str) -> Result<NaiveDateTime> {
    // Accept a handful of common shapes: full RFC3339, or "YYYY-MM-DDTHH:MM:SS".
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        return Ok(dt.naive_utc());
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp, fmt) {
            return Ok(dt);
        }
        if fmt == "%Y-%m-%d" {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(timestamp, fmt) {
                return Ok(d.and_hms_opt(0, 0, 0).expect("midnight is always valid"));
            }
        }
    }
    Err(anyhow!("could not parse timestamp {timestamp:?}"))
}

/// Enriches (location, timestamp) pairs with real historical weather data.
pub struct WeatherEnricher {
    geocoder: Geocoder,
    client: reqwest::blocking::Client,
    archive_url: String,
    cache: Mutex<HashMap<(String, String), EnrichedData>>,
    cache_capacity: usize,
    hits: Mutex<usize>,
    misses: Mutex<usize>,
}

impl WeatherEnricher {
    pub fn new(cache_size: usize) -> Self {
        Self::with_urls(
            Geocoder::new().expect("failed to build geocoder HTTP client"),
            ARCHIVE_API_URL.to_string(),
            cache_size,
        )
    }

    /// For tests: inject a geocoder and archive-API base URL pointing at a mock server.
    pub fn with_urls(geocoder: Geocoder, archive_url: String, cache_size: usize) -> Self {
        WeatherEnricher {
            geocoder,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
            archive_url,
            cache: Mutex::new(HashMap::new()),
            cache_capacity: cache_size.max(1),
            hits: Mutex::new(0),
            misses: Mutex::new(0),
        }
    }

    pub fn enrich(&self, location: &str, timestamp: &str) -> Result<EnrichedData> {
        let key = (location.to_string(), timestamp.to_string());
        if let Ok(cache) = self.cache.lock() {
            if let Some(data) = cache.get(&key) {
                if let Ok(mut hits) = self.hits.lock() {
                    *hits += 1;
                }
                return Ok(data.clone());
            }
        }
        if let Ok(mut misses) = self.misses.lock() {
            *misses += 1;
        }

        let (latitude, longitude) = self.geocoder.geocode(location)?;
        let target = parse_timestamp(timestamp)?;
        let date_str = target.format("%Y-%m-%d").to_string();

        let response = self
            .client
            .get(&self.archive_url)
            .query(&[
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
                ("start_date", date_str.clone()),
                ("end_date", date_str),
                (
                    "hourly",
                    "temperature_2m,relative_humidity_2m,weather_code".to_string(),
                ),
            ])
            .send()
            .map_err(|e| anyhow!("weather request failed for {location:?}: {e}"))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "weather service returned status {} for {location:?}",
                response.status()
            ));
        }

        let parsed: ArchiveResponse = response
            .json()
            .map_err(|e| anyhow!("failed to parse weather response for {location:?}: {e}"))?;

        let idx = nearest_hour_index(&parsed.hourly.time, &target)
            .ok_or_else(|| anyhow!("no weather data returned for {location:?} at {timestamp:?}"))?;

        let temperature = parsed
            .hourly
            .temperature_2m
            .get(idx)
            .copied()
            .flatten()
            .ok_or_else(|| anyhow!("missing temperature for {location:?} at {timestamp:?}"))?;
        let humidity = parsed
            .hourly
            .relative_humidity_2m
            .get(idx)
            .copied()
            .flatten()
            .ok_or_else(|| anyhow!("missing humidity for {location:?} at {timestamp:?}"))?;
        let weather_code = parsed
            .hourly
            .weather_code
            .get(idx)
            .copied()
            .flatten()
            .unwrap_or(-1);

        let data = EnrichedData {
            location: location.to_string(),
            latitude,
            longitude,
            temperature,
            humidity,
            condition: condition_from_wmo_code(weather_code).to_string(),
            timestamp: timestamp.to_string(),
        };

        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= self.cache_capacity {
                if let Some(k) = cache.keys().next().cloned() {
                    cache.remove(&k);
                }
            }
            cache.insert(key, data.clone());
        }

        Ok(data)
    }

    /// (hits, misses, current cache size)
    pub fn cache_stats(&self) -> (usize, usize, usize) {
        let hits = self.hits.lock().map(|h| *h).unwrap_or(0);
        let misses = self.misses.lock().map(|m| *m).unwrap_or(0);
        let size = self.cache.lock().map(|c| c.len()).unwrap_or(0);
        (hits, misses, size)
    }
}

/// Finds the hourly time-series index closest to `target`. Open-Meteo
/// returns hourly timestamps as "YYYY-MM-DDTHH:MM" with no timezone suffix.
fn nearest_hour_index(times: &[String], target: &NaiveDateTime) -> Option<usize> {
    times
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|dt| (i, (dt - *target).num_seconds().abs()))
        })
        .min_by_key(|(_, diff)| *diff)
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enricher_with_mock(server_url: String) -> WeatherEnricher {
        let geocoder = Geocoder::with_base_url(server_url.clone()).unwrap();
        WeatherEnricher::with_urls(geocoder, server_url, 100)
    }

    #[test]
    fn test_enrich_returns_real_parsed_weather() {
        let mut server = mockito::Server::new();
        let _geocode_mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/$".into()))
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "New York".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"lat": "40.7128", "lon": "-74.0060"}]"#)
            .create();

        // Same base URL serves both the geocoder mock (matched on `q`) and
        // the archive mock (matched on `latitude`) — mockito dispatches by
        // whichever registered mock's matcher succeeds.
        let _weather_mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/$".into()))
            .match_query(mockito::Matcher::UrlEncoded(
                "latitude".into(),
                "40.7128".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"hourly": {
                    "time": ["2026-01-01T00:00", "2026-01-01T01:00"],
                    "temperature_2m": [5.2, 4.8],
                    "relative_humidity_2m": [70.0, 72.0],
                    "weather_code": [1, 1]
                }}"#,
            )
            .create();

        let enricher = enricher_with_mock(server.url());
        let result = enricher.enrich("New York", "2026-01-01T00:15:00").unwrap();

        assert_eq!(result.location, "New York");
        assert!((result.latitude - 40.7128).abs() < 1e-6);
        assert!((result.temperature - 5.2).abs() < 1e-6); // nearest hour: 00:00
        assert_eq!(result.condition, "Mainly Clear");
    }

    #[test]
    fn test_enrich_caches_repeated_lookups() {
        let mut server = mockito::Server::new();
        let _geocode_mock = server
            .mock("GET", mockito::Matcher::Any)
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "Paris".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"lat": "48.8566", "lon": "2.3522"}]"#)
            .create();
        let weather_mock = server
            .mock("GET", mockito::Matcher::Any)
            .match_query(mockito::Matcher::UrlEncoded(
                "latitude".into(),
                "48.8566".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"hourly": {"time": ["2026-01-01T00:00"], "temperature_2m": [3.0], "relative_humidity_2m": [80.0], "weather_code": [0]}}"#,
            )
            .expect(1)
            .create();

        let enricher = enricher_with_mock(server.url());
        enricher.enrich("Paris", "2026-01-01T00:00:00").unwrap();
        enricher.enrich("Paris", "2026-01-01T00:00:00").unwrap();

        weather_mock.assert();
        let (hits, misses, size) = enricher.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(size, 1);
    }

    #[test]
    fn test_wmo_code_mapping_is_real_not_arbitrary() {
        assert_eq!(condition_from_wmo_code(0), "Clear");
        assert_eq!(condition_from_wmo_code(61), "Rain");
        assert_eq!(condition_from_wmo_code(95), "Thunderstorm");
        assert_eq!(condition_from_wmo_code(71), "Snow");
    }

    #[test]
    fn test_nearest_hour_index_picks_closest() {
        let times = vec![
            "2026-01-01T00:00".to_string(),
            "2026-01-01T06:00".to_string(),
            "2026-01-01T12:00".to_string(),
        ];
        let target = NaiveDateTime::parse_from_str("2026-01-01T07:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        assert_eq!(nearest_hour_index(&times, &target), Some(1));
    }

    #[test]
    fn test_parse_timestamp_accepts_common_formats() {
        assert!(parse_timestamp("2026-01-01T00:00:00Z").is_ok());
        assert!(parse_timestamp("2026-01-01T00:00:00").is_ok());
        assert!(parse_timestamp("2026-01-01").is_ok());
        assert!(parse_timestamp("not a date").is_err());
    }
}
