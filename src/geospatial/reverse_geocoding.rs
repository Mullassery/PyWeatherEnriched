/// Reverse Geocoding Module - Latitude/Longitude to Postal Code + Address
///
/// CRITICAL: OpenStreetMap-based reverse geocoding (always available)
/// OPTIONAL: Google Maps, USPS database (framework stubs)
///
/// Features:
/// - Postal code lookup from coordinates
/// - Full address extraction (street, city, state, country)
/// - Administrative boundary information
/// - Configurable output detail levels
/// - Multiple data source support with auto-detection
/// - LRU caching for performance
/// - Batch processing support

use crate::geospatial::config::DataSourceConfig;
use crate::geospatial::data_source::{create_loader, GeoDataLoader};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Output detail level - user chooses what information to return
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OutputDetailLevel {
    /// Postal code only
    Minimal,
    /// Postal code + basic address (street, city, state, country)
    Standard,
    /// Standard + admin boundaries (county, neighborhood)
    Extended,
    /// Everything + alternatives + metadata
    Complete,
}

/// Primary reverse geocoding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseGeocodeResult {
    pub postal_code: String,
    pub postal_code_type: String, // "ZIP", "POSTCODE", etc
    pub street_address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: String,
    pub country_code: String,
    pub admin_level_1: Option<String>, // State/Province
    pub admin_level_2: Option<String>, // County/District
    pub neighborhood: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub confidence: f32, // 0.0-1.0
    pub source: String, // "osm", "google", "usps"
}

/// Complete response with alternatives and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteReverseGeocodeResponse {
    pub primary: ReverseGeocodeResult,
    pub alternatives: Vec<ReverseGeocodeResult>,
    pub sources_tried: Vec<String>,
    pub processing_time_ms: u64,
}

/// Minimal response (postal code only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalReverseGeocodeResult {
    pub postal_code: String,
    pub postal_code_type: String,
    pub country: String,
    pub confidence: f32,
}

/// Standard response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardReverseGeocodeResult {
    pub postal_code: String,
    pub postal_code_type: String,
    pub street_address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: String,
    pub country_code: String,
    pub confidence: f32,
    pub source: String,
}

/// Extended response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedReverseGeocodeResult {
    pub postal_code: String,
    pub postal_code_type: String,
    pub street_address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: String,
    pub country_code: String,
    pub admin_level_1: Option<String>,
    pub admin_level_2: Option<String>,
    pub neighborhood: Option<String>,
    pub confidence: f32,
    pub source: String,
}

/// Approximate (lon, lat) centroid of a GeoJSON feature's geometry — the
/// mean of all coordinate vertices. Exact for Points; a reasonable
/// approximation (not a true area-weighted centroid) for Polygons/
/// LineStrings, which is sufficient for "nearest feature" ranking.
fn feature_centroid(feature: &geojson::Feature) -> Option<(f64, f64)> {
    let geometry = feature.geometry.as_ref()?;
    let mut sum_lon = 0.0;
    let mut sum_lat = 0.0;
    let mut count = 0usize;
    collect_points(&geometry.value, &mut sum_lon, &mut sum_lat, &mut count);
    if count == 0 {
        None
    } else {
        Some((sum_lon / count as f64, sum_lat / count as f64))
    }
}

fn collect_points(value: &geojson::Value, sum_lon: &mut f64, sum_lat: &mut f64, count: &mut usize) {
    use geojson::GeometryValue::*;
    match value {
        Point { coordinates: p } => {
            if p.len() >= 2 {
                *sum_lon += p[0];
                *sum_lat += p[1];
                *count += 1;
            }
        }
        MultiPoint { coordinates: points } | LineString { coordinates: points } => {
            for p in points {
                if p.len() >= 2 {
                    *sum_lon += p[0];
                    *sum_lat += p[1];
                    *count += 1;
                }
            }
        }
        MultiLineString { coordinates: lines } | Polygon { coordinates: lines } => {
            for line in lines {
                for p in line {
                    if p.len() >= 2 {
                        *sum_lon += p[0];
                        *sum_lat += p[1];
                        *count += 1;
                    }
                }
            }
        }
        MultiPolygon { coordinates: polys } => {
            for poly in polys {
                for line in poly {
                    for p in line {
                        if p.len() >= 2 {
                            *sum_lon += p[0];
                            *sum_lat += p[1];
                            *count += 1;
                        }
                    }
                }
            }
        }
        GeometryCollection { geometries: geoms } => {
            for g in geoms {
                collect_points(&g.value, sum_lon, sum_lat, count);
            }
        }
    }
}

/// Great-circle distance in kilometers (Haversine formula).
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS_KM * c
}

/// Reverse geocoding service
pub struct ReverseGeocodingService {
    osm_loader: Arc<dyn GeoDataLoader>,
    cache: Arc<std::sync::Mutex<lru::LruCache<String, ReverseGeocodeResult>>>,
    cache_enabled: bool,
}

impl ReverseGeocodingService {
    /// Create reverse geocoding service with OSM data source
    pub fn new(config: &DataSourceConfig, cache_enabled: bool) -> Result<Self> {
        let osm_loader = create_loader(&config.source)?;

        let cache = Arc::new(std::sync::Mutex::new(
            lru::LruCache::new(std::num::NonZeroUsize::new(5000).unwrap()),
        ));

        Ok(ReverseGeocodingService {
            osm_loader,
            cache,
            cache_enabled,
        })
    }

    /// Reverse geocode a single location - returns full result
    pub fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        let cache_key = format!("rgeo_{:.6}_{:.6}", latitude, longitude);

        // Check cache
        if self.cache_enabled {
            if let Ok(mut cache) = self.cache.lock() {
                if let Some(result) = cache.get(&cache_key) {
                    return Ok(result.clone());
                }
            }
        }

        // Load from OSM
        let result = self.reverse_geocode_osm(latitude, longitude)?;

        // Cache result
        if self.cache_enabled {
            if let Ok(mut cache) = self.cache.lock() {
                cache.put(cache_key, result.clone());
            }
        }

        Ok(result)
    }

    /// Reverse geocode with configurable output detail level
    pub fn reverse_geocode_with_detail(
        &self,
        latitude: f64,
        longitude: f64,
        detail_level: OutputDetailLevel,
    ) -> Result<String> {
        let full_result = self.reverse_geocode(latitude, longitude)?;

        let output = match detail_level {
            OutputDetailLevel::Minimal => {
                serde_json::to_string(&MinimalReverseGeocodeResult {
                    postal_code: full_result.postal_code.clone(),
                    postal_code_type: full_result.postal_code_type.clone(),
                    country: full_result.country.clone(),
                    confidence: full_result.confidence,
                })?
            }
            OutputDetailLevel::Standard => {
                serde_json::to_string(&StandardReverseGeocodeResult {
                    postal_code: full_result.postal_code.clone(),
                    postal_code_type: full_result.postal_code_type.clone(),
                    street_address: full_result.street_address.clone(),
                    city: full_result.city.clone(),
                    state: full_result.state.clone(),
                    country: full_result.country.clone(),
                    country_code: full_result.country_code.clone(),
                    confidence: full_result.confidence,
                    source: full_result.source.clone(),
                })?
            }
            OutputDetailLevel::Extended => {
                serde_json::to_string(&ExtendedReverseGeocodeResult {
                    postal_code: full_result.postal_code.clone(),
                    postal_code_type: full_result.postal_code_type.clone(),
                    street_address: full_result.street_address.clone(),
                    city: full_result.city.clone(),
                    state: full_result.state.clone(),
                    country: full_result.country.clone(),
                    country_code: full_result.country_code.clone(),
                    admin_level_1: full_result.admin_level_1.clone(),
                    admin_level_2: full_result.admin_level_2.clone(),
                    neighborhood: full_result.neighborhood.clone(),
                    confidence: full_result.confidence,
                    source: full_result.source.clone(),
                })?
            }
            OutputDetailLevel::Complete => serde_json::to_string(&CompleteReverseGeocodeResponse {
                primary: full_result,
                alternatives: Vec::new(), // TODO: Get alternatives from multiple sources
                sources_tried: vec!["osm".to_string()],
                processing_time_ms: 0, // TODO: Track timing
            })?,
        };

        Ok(output)
    }

    /// Batch reverse geocoding for multiple coordinates
    pub fn reverse_geocode_batch(
        &self,
        coordinates: Vec<(f64, f64)>,
    ) -> Result<Vec<ReverseGeocodeResult>> {
        let mut results = Vec::new();

        for (lat, lon) in coordinates {
            match self.reverse_geocode(lat, lon) {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Log error but continue processing
                    eprintln!("Failed to reverse geocode {},{}: {}", lat, lon, e);
                }
            }
        }

        Ok(results)
    }

    /// Real reverse geocoding: parses the OSM GeoJSON FeatureCollection for
    /// this tile, finds the feature nearest to (latitude, longitude), and
    /// extracts real address fields from its properties (checking the
    /// common OSM property key variants). Returns an error — not a
    /// fabricated address — if the tile has no usable features.
    fn reverse_geocode_osm(&self, latitude: f64, longitude: f64) -> Result<ReverseGeocodeResult> {
        let tile_id = self.get_tile_id(latitude, longitude);
        let osm_json = self.osm_loader.get_vector(&tile_id)?;

        let collection: geojson::FeatureCollection = osm_json
            .parse::<geojson::GeoJson>()
            .map_err(|e| anyhow::anyhow!("failed to parse OSM GeoJSON for tile {tile_id}: {e}"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("tile {tile_id} is not a GeoJSON FeatureCollection: {e}"))?;

        let nearest = collection
            .features
            .iter()
            .filter_map(|f| {
                let centroid = feature_centroid(f)?;
                let dist = haversine_km(latitude, longitude, centroid.1, centroid.0);
                Some((dist, f))
            })
            .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| {
                anyhow::anyhow!("no usable features found in tile {tile_id} for reverse geocoding")
            })?;

        let (distance_km, feature) = nearest;
        let props = feature.properties.as_ref();

        let get = |keys: &[&str]| -> Option<String> {
            let props = props?;
            keys.iter()
                .find_map(|k| props.get(*k))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };

        let postal_code = get(&["addr:postcode", "postal_code", "postcode", "zip"])
            .ok_or_else(|| {
                anyhow::anyhow!("nearest feature in tile {tile_id} has no postal code field")
            })?;

        // Confidence decays with distance to the matched feature — an exact
        // hit (0km) is high-confidence; a match several km away is not.
        let confidence = (1.0 - (distance_km / 5.0)).clamp(0.1, 1.0) as f32;

        Ok(ReverseGeocodeResult {
            postal_code,
            postal_code_type: "POSTCODE".to_string(),
            street_address: get(&["addr:full", "addr:street"]),
            city: get(&["addr:city", "city"]),
            state: get(&["addr:state", "state"]),
            country: get(&["addr:country", "country"]).unwrap_or_else(|| "Unknown".to_string()),
            country_code: get(&["addr:country", "ISO3166-2", "country_code"])
                .unwrap_or_else(|| "??".to_string()),
            admin_level_1: get(&["addr:state", "state"]),
            admin_level_2: get(&["addr:county", "county"]),
            neighborhood: get(&["addr:neighbourhood", "addr:neighborhood", "neighbourhood"]),
            latitude,
            longitude,
            confidence,
            source: "osm".to_string(),
        })
    }

    /// Get tile ID from coordinates
    fn get_tile_id(&self, latitude: f64, longitude: f64) -> String {
        let lat = latitude.floor() as i32;
        let lon = longitude.floor() as i32;
        format!("{}_{}", lat, lon)
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.cache.lock() {
            (cache.len(), 5000) // (current_size, max_size)
        } else {
            (0, 5000)
        }
    }
}

/// Batch processor for multiple locations
pub struct BatchReverseGeocoder {
    service: Arc<ReverseGeocodingService>,
}

impl BatchReverseGeocoder {
    pub fn new(service: Arc<ReverseGeocodingService>) -> Self {
        BatchReverseGeocoder { service }
    }

    /// Process batch with progress tracking
    pub fn process_with_progress(
        &self,
        coordinates: Vec<(f64, f64, String)>, // lat, lon, identifier
    ) -> Result<Vec<BatchGeocodeResult>> {
        let mut results = Vec::new();

        for (lat, lon, id) in coordinates {
            match self.service.reverse_geocode(lat, lon) {
                Ok(geocode_result) => {
                    results.push(BatchGeocodeResult {
                        identifier: id,
                        latitude: lat,
                        longitude: lon,
                        result: Some(geocode_result),
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(BatchGeocodeResult {
                        identifier: id,
                        latitude: lat,
                        longitude: lon,
                        result: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGeocodeResult {
    pub identifier: String,
    pub latitude: f64,
    pub longitude: f64,
    pub result: Option<ReverseGeocodeResult>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postal_code_format() {
        let result = ReverseGeocodeResult {
            postal_code: "10001".to_string(),
            postal_code_type: "ZIP".to_string(),
            street_address: Some("350 5th Ave".to_string()),
            city: Some("New York".to_string()),
            state: Some("NY".to_string()),
            country: "US".to_string(),
            country_code: "US".to_string(),
            admin_level_1: Some("New York".to_string()),
            admin_level_2: Some("New York County".to_string()),
            neighborhood: Some("Koreatown".to_string()),
            latitude: 40.7128,
            longitude: -74.0060,
            confidence: 0.95,
            source: "osm".to_string(),
        };

        assert_eq!(result.postal_code, "10001");
        assert_eq!(result.city, Some("New York".to_string()));
        assert_eq!(result.country_code, "US");
    }

    #[test]
    fn test_output_serialization() {
        let result = MinimalReverseGeocodeResult {
            postal_code: "10001".to_string(),
            postal_code_type: "ZIP".to_string(),
            country: "US".to_string(),
            confidence: 0.95,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("10001"));
        assert!(json.contains("\"confidence\":0.95"));
    }

    fn service_with_geojson_fixture(dir: &std::path::Path, geojson_body: &str) -> ReverseGeocodingService {
        std::fs::write(dir.join("40_-74.geojson"), geojson_body).unwrap();
        let loader: Arc<dyn GeoDataLoader> = Arc::new(super::super::data_source::LocalFileLoader::new(
            dir.to_path_buf(),
            "{lat}_{lon}.geojson".to_string(),
        ));
        ReverseGeocodingService {
            osm_loader: loader,
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(100).unwrap()),
            )),
            cache_enabled: true,
        }
    }

    #[test]
    fn test_reverse_geocode_extracts_real_properties_not_hardcoded_nyc() {
        let dir = tempfile::tempdir().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [-73.9857, 40.7484]},
                    "properties": {
                        "addr:postcode": "94103",
                        "addr:city": "San Francisco",
                        "addr:state": "CA",
                        "addr:country": "US"
                    }
                }
            ]
        }"#;
        let service = service_with_geojson_fixture(dir.path(), geojson);

        let result = service.reverse_geocode(40.7484, -73.9857).unwrap();

        // The fixture's own (deliberately not-NYC-looking) postal code and
        // city must come through — proving this is real extraction, not
        // the previous hardcoded "350 5th Ave, New York, Koreatown" output.
        assert_eq!(result.postal_code, "94103");
        assert_eq!(result.city, Some("San Francisco".to_string()));
        assert_eq!(result.state, Some("CA".to_string()));
        assert_ne!(result.neighborhood, Some("Koreatown".to_string()));
    }

    #[test]
    fn test_reverse_geocode_picks_nearest_of_multiple_features() {
        let dir = tempfile::tempdir().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [-73.5, 40.5]},
                    "properties": {"addr:postcode": "AAAAA", "addr:city": "Far"}
                },
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [-73.501, 40.501]},
                    "properties": {"addr:postcode": "BBBBB", "addr:city": "Near"}
                }
            ]
        }"#;
        let service = service_with_geojson_fixture(dir.path(), geojson);

        // Query point (still well inside the same 40_-74 tile the fixture
        // helper writes to) is much closer to the second feature.
        let result = service.reverse_geocode(40.5009, -73.5009).unwrap();

        assert_eq!(result.postal_code, "BBBBB");
        assert_eq!(result.city, Some("Near".to_string()));
    }

    #[test]
    fn test_reverse_geocode_no_postal_code_field_is_a_real_error() {
        let dir = tempfile::tempdir().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [-74.0, 40.0]},
                    "properties": {"addr:city": "Nowhere Specific"}
                }
            ]
        }"#;
        let service = service_with_geojson_fixture(dir.path(), geojson);

        let result = service.reverse_geocode(40.0, -74.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_reverse_geocode_empty_feature_collection_is_a_real_error() {
        let dir = tempfile::tempdir().unwrap();
        let geojson = r#"{"type": "FeatureCollection", "features": []}"#;
        let service = service_with_geojson_fixture(dir.path(), geojson);

        let result = service.reverse_geocode(40.0, -74.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_feature_centroid_polygon_averages_ring_vertices() {
        let feature: geojson::Feature = serde_json::from_str(
            r#"{
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0, 0.0], [0.0, 2.0], [2.0, 2.0], [2.0, 0.0], [0.0, 0.0]]]
                },
                "properties": {}
            }"#,
        )
        .unwrap();

        let centroid = feature_centroid(&feature).unwrap();
        // Average of the 5 ring vertices (closing vertex repeats (0,0)).
        assert!((centroid.0 - 0.8).abs() < 0.01);
        assert!((centroid.1 - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_haversine_km_known_distance() {
        // New York to Los Angeles is ~3936 km.
        let d = haversine_km(40.7128, -74.0060, 34.0522, -118.2437);
        assert!((d - 3936.0).abs() < 50.0);
    }
}
