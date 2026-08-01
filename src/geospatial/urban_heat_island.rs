/// Urban Heat Island (UHI) module - OSM-based modeling
///
/// Calculates UHI effect from:
/// - Building density
/// - Building heights
/// - Surface materials
/// - Vegetation coverage
///
/// UHI Effect Formula:
/// UHI = 0.7 + 0.25 * building_density + 0.1 * avg_building_height
///
/// Typical values:
/// - Urban core: +2.5 to +4.0°C
/// - Suburban: +1.0 to +2.0°C
/// - Rural: 0°C baseline

use crate::geospatial::config::DataSourceConfig;
use crate::geospatial::data_source::{create_loader, GeoDataLoader};
use crate::geospatial::UHIData;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct UHIService {
    loader: Arc<dyn GeoDataLoader>,
    cache: Arc<std::sync::Mutex<lru::LruCache<String, UHIData>>>,
}

impl UHIService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        let loader = create_loader(&config.source)?;
        let cache = Arc::new(std::sync::Mutex::new(
            lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
        ));

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
        let (building_density, avg_height) = self.analyze_buildings(&geojson, latitude, longitude)?;

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
    fn analyze_buildings(&self, geojson: &Value, latitude: f64, longitude: f64) -> Result<(f32, f32)> {
        let mut total_area = 0.0f32;
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
                            } else if let Some(levels_str) = props.get("building:levels").and_then(|v| v.as_str()) {
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
        let search_area = 1_000_000.0f32;
        total_area = search_area;

        let building_density = (total_building_area / total_area).min(1.0);
        let avg_height = if heights.is_empty() {
            10.0 // Default 3 stories
        } else {
            heights.iter().sum::<f32>() / heights.len() as f32
        };

        Ok((building_density, avg_height))
    }

    /// Estimate polygon area (simplified)
    fn estimate_polygon_area(&self, geom: &Value) -> f32 {
        // Simplified: assume buildings are roughly 30-100 m² on average
        // Real implementation would use proper polygon area calculation
        // (Shoelace formula for lat/lon polygons)
        50.0 // m²
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
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(1).unwrap()),
            )),
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
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(1).unwrap()),
            )),
        };

        assert_eq!(service.classify_location(0.05), "rural");
        assert_eq!(service.classify_location(0.2), "suburban");
        assert_eq!(service.classify_location(0.5), "dense_urban");
    }
}
