//! Forward geocoding: location name -> (latitude, longitude).
//!
//! Uses OpenStreetMap's Nominatim search API (nominatim.openstreetmap.org),
//! a real, free, no-API-key public geocoding service. Nominatim's usage
//! policy requires a descriptive User-Agent and reasonable request rates —
//! both respected here.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";
const USER_AGENT: &str = "pyweatherenriched/0.4 (weather data enrichment; contact via GitHub)";

#[derive(Debug, Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
}

/// Forward geocodes location strings to coordinates via Nominatim, with an
/// in-memory cache (the same location string is looked up repeatedly across
/// a batch far more often than it changes).
pub struct Geocoder {
    client: reqwest::blocking::Client,
    base_url: String,
    cache: Mutex<HashMap<String, (f64, f64)>>,
}

impl Geocoder {
    pub fn new() -> Result<Self> {
        Self::with_base_url(NOMINATIM_URL.to_string())
    }

    /// For tests: point at a mock server instead of the real Nominatim endpoint.
    pub fn with_base_url(base_url: String) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Geocoder {
            client,
            base_url,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Geocode a location string (e.g. "New York" or "Paris, France") to
    /// (latitude, longitude). Returns an error — not fabricated
    /// coordinates — when the location can't be resolved.
    pub fn geocode(&self, location: &str) -> Result<(f64, f64)> {
        if let Ok(cache) = self.cache.lock() {
            if let Some(&coords) = cache.get(location) {
                return Ok(coords);
            }
        }

        let response = self
            .client
            .get(&self.base_url)
            .query(&[("q", location), ("format", "json"), ("limit", "1")])
            .send()
            .map_err(|e| anyhow!("geocoding request failed for {location:?}: {e}"))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "geocoding service returned status {} for {location:?}",
                response.status()
            ));
        }

        let results: Vec<NominatimResult> = response
            .json()
            .map_err(|e| anyhow!("failed to parse geocoding response for {location:?}: {e}"))?;

        let first = results
            .first()
            .ok_or_else(|| anyhow!("no geocoding results for location {location:?}"))?;

        let lat: f64 = first
            .lat
            .parse()
            .map_err(|_| anyhow!("invalid latitude in geocoding response: {:?}", first.lat))?;
        let lon: f64 = first
            .lon
            .parse()
            .map_err(|_| anyhow!("invalid longitude in geocoding response: {:?}", first.lon))?;

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(location.to_string(), (lat, lon));
        }

        Ok((lat, lon))
    }
}

impl Default for Geocoder {
    fn default() -> Self {
        Self::new().expect("failed to build HTTP client for Geocoder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocode_returns_real_parsed_coordinates() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "New York".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"lat": "40.7128", "lon": "-74.0060"}]"#)
            .create();

        let geocoder = Geocoder::with_base_url(server.url()).unwrap();
        let (lat, lon) = geocoder.geocode("New York").unwrap();

        assert!((lat - 40.7128).abs() < 1e-6);
        assert!((lon - (-74.0060)).abs() < 1e-6);
    }

    #[test]
    fn test_geocode_caches_repeated_lookups() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"lat": "48.8566", "lon": "2.3522"}]"#)
            .expect(1) // only one real request even though we geocode twice
            .create();

        let geocoder = Geocoder::with_base_url(server.url()).unwrap();
        geocoder.geocode("Paris").unwrap();
        geocoder.geocode("Paris").unwrap();

        mock.assert();
    }

    #[test]
    fn test_geocode_unresolvable_location_is_a_real_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create();

        let geocoder = Geocoder::with_base_url(server.url()).unwrap();
        let result = geocoder.geocode("Nowhereville, Atlantis");

        assert!(result.is_err());
    }
}
