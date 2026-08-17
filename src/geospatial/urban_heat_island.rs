//! Urban Heat Island (UHI) module - OSM-based modeling
//!
//! Calculates UHI effect from:
//! - Building density
//! - Building heights
//! - Surface materials
//! - Vegetation coverage
//!
//! UHI Effect Formula:
//! UHI = 0.7 + 0.25 * building_density + 0.1 * avg_building_height
//!
//! Typical values:
//! - Urban core: +2.5 to +4.0°C
//! - Suburban: +1.0 to +2.0°C
//! - Rural: 0°C baseline

use crate::geospatial::config::DataSourceConfig;
use crate::geospatial::data_source::{create_loader, GeoDataLoader};
use crate::geospatial::UHIData;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

pub struct UHIService {
    loader: Arc<dyn GeoDataLoader>,
    cache: Arc<std::sync::Mutex<lru::LruCache<String, UHIData>>>,
}

impl UHIService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        let loader = create_loader(&config.source)?;
        let cache = Arc::new(std::sync::Mutex::new(lru::LruCache::new(
            std::num::NonZeroUsize::new(1000).unwrap(),
        )));

        Ok(UHIService { loader, cache })
    }

    /// Get UHI data for a location
    pub fn get_uhi_data(&self, latitude: f64, longitude: f64) -> Result<UHIData> {
        let cache_key = format!("uhi_{:.4}_{:.4}", latitude, longitude);

        // Check cache
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(data) = cache.get(&cache_key) {
                return Ok(data.clone());
            }
        }

        // Load OSM data
        let tile_id = self.get_tile_id(latitude, longitude);
        let uhi_data = self.load_uhi_from_osm(latitude, longitude, &tile_id)?;

        // Cache result
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(cache_key, uhi_data.clone());
        }

        Ok(uhi_data)
    }

    /// Load UHI data from OSM GeoJSON
    fn load_uhi_from_osm(&self, latitude: f64, longitude: f64, tile_id: &str) -> Result<UHIData> {
        // Get OSM GeoJSON for tile
        let osm_json = self.loader.get_vector(tile_id)?;
        let geojson: Value = serde_json::from_str(&osm_json)?;

        // Extract building features
        let (building_density, avg_height) =
            self.analyze_buildings(&geojson, latitude, longitude)?;

        // Calculate UHI effect
        let uhi_effect = self.calculate_uhi_effect(building_density, avg_height);
        let location_type = self.classify_location(building_density);

        Ok(UHIData {
            building_density_percent: building_density * 100.0,
            average_building_height_m: avg_height,
            uhi_effect_c: uhi_effect,
            location_type,
        })
    }

    /// Analyze buildings from OSM GeoJSON
    fn analyze_buildings(
        &self,
        geojson: &Value,
        _latitude: f64,
        _longitude: f64,
    ) -> Result<(f32, f32)> {
        let mut total_building_area = 0.0f32;
        let mut heights = Vec::new();

        if let Some(features) = geojson["features"].as_array() {
            for feature in features {
                if let Some(props) = feature["properties"].as_object() {
                    // Check if building
                    if props.get("building").is_some() {
                        // Get area from geometry
                        if let Some(geom) = feature["geometry"].as_object() {
                            let area = self.estimate_polygon_area(geom);
                            total_building_area += area;

                            // Get building height if available
                            if let Some(height_str) = props.get("height").and_then(|v| v.as_str()) {
                                if let Ok(height) = height_str.parse::<f32>() {
                                    heights.push(height);
                                }
                            } else if let Some(levels_str) =
                                props.get("building:levels").and_then(|v| v.as_str())
                            {
                                if let Ok(levels) = levels_str.parse::<f32>() {
                                    heights.push(levels * 3.5); // Assume 3.5m per level
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate search area (1km x 1km = 1,000,000 m²)
        let total_area = 1_000_000.0f32;

        let building_density = (total_building_area / total_area).min(1.0);
        let avg_height = if heights.is_empty() {
            10.0 // Default 3 stories
        } else {
            heights.iter().sum::<f32>() / heights.len() as f32
        };

        Ok((building_density, avg_height))
    }

    /// Real polygon area in square meters via the Shoelace formula, with an
    /// equirectangular projection (centered on the polygon's own latitude)
    /// to convert degree-based coordinates to meters — accurate for
    /// building-footprint scales (tens to low-thousands of m²), where
    /// projection distortion over such a small extent is negligible.
    /// Falls back to 0.0 (not a fabricated guess) when geometry is missing
    /// or malformed.
    fn estimate_polygon_area(&self, geom: &serde_json::Map<String, Value>) -> f32 {
        let geometry_type = geom.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let coordinates = match geom.get("coordinates") {
            Some(c) => c,
            None => return 0.0,
        };

        // Only Polygon and the first polygon of a MultiPolygon are counted;
        // buildings are overwhelmingly simple Polygons in OSM exports.
        let outer_ring = match geometry_type {
            "Polygon" => coordinates.as_array().and_then(|rings| rings.first()),
            "MultiPolygon" => coordinates
                .as_array()
                .and_then(|polys| polys.first())
                .and_then(|poly| poly.as_array())
                .and_then(|rings| rings.first()),
            _ => return 0.0,
        };

        let Some(ring) = outer_ring.and_then(|r| r.as_array()) else {
            return 0.0;
        };

        let points: Vec<(f64, f64)> = ring
            .iter()
            .filter_map(|pt| {
                let arr = pt.as_array()?;
                let lon = arr.first()?.as_f64()?;
                let lat = arr.get(1)?.as_f64()?;
                Some((lon, lat))
            })
            .collect();

        if points.len() < 3 {
            return 0.0;
        }

        const EARTH_RADIUS_M: f64 = 6_371_000.0;
        let mean_lat_rad = points.iter().map(|(_, lat)| lat).sum::<f64>() / points.len() as f64;
        let meters_per_deg_lat = EARTH_RADIUS_M * std::f64::consts::PI / 180.0;
        let meters_per_deg_lon = meters_per_deg_lat * mean_lat_rad.to_radians().cos();

        let projected: Vec<(f64, f64)> = points
            .iter()
            .map(|(lon, lat)| (lon * meters_per_deg_lon, lat * meters_per_deg_lat))
            .collect();

        // Shoelace formula.
        let mut sum = 0.0;
        for i in 0..projected.len() {
            let (x1, y1) = projected[i];
            let (x2, y2) = projected[(i + 1) % projected.len()];
            sum += x1 * y2 - x2 * y1;
        }
        (sum.abs() / 2.0) as f32
    }

    /// Calculate UHI effect in °C
    /// Formula: UHI = 0.7 + 0.25 * building_density + 0.1 * avg_height
    fn calculate_uhi_effect(&self, building_density: f32, avg_height: f32) -> f32 {
        0.7 + (0.25 * building_density) + (0.1 * avg_height.min(100.0) / 100.0)
    }

    /// Classify location type
    fn classify_location(&self, building_density: f32) -> String {
        match building_density {
            d if d < 0.1 => "rural".to_string(),
            d if d < 0.3 => "suburban".to_string(),
            d if d < 0.6 => "dense_urban".to_string(),
            _ => "dense_urban_core".to_string(),
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uhi_calculation() {
        let service = UHIService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                std::path::PathBuf::from("/tmp"),
                "test.geojson".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(1).unwrap(),
            ))),
        };

        // Dense urban: 85% building density, avg 25m height
        let uhi = service.calculate_uhi_effect(0.85, 25.0);
        // 0.7 + (0.25 * 0.85) + (0.1 * 0.25) = 0.7 + 0.2125 + 0.025 = 0.9375°C
        assert!((uhi - 0.9375).abs() < 0.01);
    }

    #[test]
    fn test_location_classification() {
        let service = UHIService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                std::path::PathBuf::from("/tmp"),
                "test.geojson".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(1).unwrap(),
            ))),
        };

        assert_eq!(service.classify_location(0.05), "rural");
        assert_eq!(service.classify_location(0.2), "suburban");
        assert_eq!(service.classify_location(0.5), "dense_urban");
    }

    fn uhi_service() -> UHIService {
        UHIService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                std::path::PathBuf::from("/tmp"),
                "test.geojson".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(1).unwrap(),
            ))),
        }
    }

    #[test]
    fn test_estimate_polygon_area_computes_real_shoelace_area_not_hardcoded_50() {
        let service = uhi_service();

        // A ~0.001deg square near the equator (longitude scale distortion
        // is negligible there), real area via Shoelace + equirectangular
        // projection should land near 111m x 111m =~ 12,300 m^2 -- nothing
        // at all like the old hardcoded 50.0 regardless of geometry.
        let geom: serde_json::Map<String, Value> = serde_json::from_str(
            r#"{
                "type": "Polygon",
                "coordinates": [[[0.0, 0.0], [0.001, 0.0], [0.001, 0.001], [0.0, 0.001], [0.0, 0.0]]]
            }"#,
        )
        .unwrap();

        let area = service.estimate_polygon_area(&geom);

        assert!(
            (10_000.0..15_000.0).contains(&area),
            "expected a real geometric area near ~12,300 m^2, got {area}"
        );
    }

    #[test]
    fn test_estimate_polygon_area_scales_with_actual_size() {
        let service = uhi_service();

        let small: serde_json::Map<String, Value> = serde_json::from_str(
            r#"{"type": "Polygon", "coordinates": [[[0.0, 0.0], [0.0005, 0.0], [0.0005, 0.0005], [0.0, 0.0005], [0.0, 0.0]]]}"#,
        )
        .unwrap();
        let large: serde_json::Map<String, Value> = serde_json::from_str(
            r#"{"type": "Polygon", "coordinates": [[[0.0, 0.0], [0.002, 0.0], [0.002, 0.002], [0.0, 0.002], [0.0, 0.0]]]}"#,
        )
        .unwrap();

        let small_area = service.estimate_polygon_area(&small);
        let large_area = service.estimate_polygon_area(&large);

        // A polygon with 4x the side length should have ~16x the area —
        // real geometric scaling, unlike the old constant-50.0 stub where
        // every shape "had" the same area.
        assert!(large_area > small_area * 10.0);
    }

    #[test]
    fn test_estimate_polygon_area_malformed_geometry_returns_zero_not_a_guess() {
        let service = uhi_service();
        let geom: serde_json::Map<String, Value> =
            serde_json::from_str(r#"{"type": "Polygon", "coordinates": [[]]}"#).unwrap();

        assert_eq!(service.estimate_polygon_area(&geom), 0.0);
    }
}
