/// Elevation module - SRTM DEM processing
///
/// Provides:
/// - Elevation lookup at any coordinate
/// - Lapse rate temperature adjustment (-0.65°C per 100m)
/// - Terrain roughness for wind calculations
/// - Elevation categorization

use crate::geospatial::config::DataSourceConfig;
use crate::geospatial::data_source::{create_loader, GeoDataLoader};
use crate::geospatial::ElevationData;
use anyhow::Result;
use std::sync::Arc;

pub struct ElevationService {
    loader: Arc<dyn GeoDataLoader>,
    cache: Arc<std::sync::Mutex<lru::LruCache<String, f32>>>,
}

impl ElevationService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        let loader = create_loader(&config.source)?;
        let cache = Arc::new(std::sync::Mutex::new(
            lru::LruCache::new(std::num::NonZeroUsize::new(10000).unwrap()),
        ));

        Ok(ElevationService { loader, cache })
    }

    /// Get elevation at coordinate
    pub fn get_elevation(&self, latitude: f64, longitude: f64) -> Result<ElevationData> {
        let cache_key = format!("elev_{:.4}_{:.4}", latitude, longitude);

        // Check cache
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(elevation_m) = cache.get(&cache_key) {
                return Ok(ElevationData {
                    elevation_m: *elevation_m,
                    lapse_rate_adjustment_c: self.calculate_lapse_rate(*elevation_m),
                    terrain_roughness: self.calculate_roughness(*elevation_m),
                });
            }
        }

        // Get tile ID from coordinates
        let tile_id = self.get_tile_id(latitude, longitude);

        // Load from data source
        let elevation_m = self.load_elevation_from_tile(latitude, longitude, &tile_id)?;

        // Cache result
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(cache_key, elevation_m);
        }

        Ok(ElevationData {
            elevation_m,
            lapse_rate_adjustment_c: self.calculate_lapse_rate(elevation_m),
            terrain_roughness: self.calculate_roughness(elevation_m),
        })
    }

    /// Calculate temperature adjustment due to elevation
    /// Standard lapse rate: -0.65°C per 100m
    /// Reference altitude: sea level (0m)
    fn calculate_lapse_rate(&self, elevation_m: f32) -> f32 {
        const LAPSE_RATE: f32 = -0.0065; // °C per meter
        elevation_m * LAPSE_RATE
    }

    /// Calculate terrain roughness for wind modeling
    /// Rougher terrain (mountains) = higher roughness
    /// Flat terrain = lower roughness
    fn calculate_roughness(&self, elevation_m: f32) -> f32 {
        // Simple model: higher elevation = more exposed = lower roughness
        // Lower elevation = valleys/sheltered = higher roughness (but for wind, roughness is buildings/trees, not terrain)

        // For pure terrain:
        // - High mountains (>2000m): z0 ≈ 0.5m (rocky, sparse vegetation)
        // - Low hills (500-2000m): z0 ≈ 0.2m
        // - Flat terrain (0-500m): z0 ≈ 0.1m (grassland)

        match elevation_m {
            elev if elev > 2000.0 => 0.5,
            elev if elev > 500.0 => 0.2,
            _ => 0.1,
        }
    }

    /// Get tile ID from coordinates
    /// SRTM is organized in 1°×1° tiles
    /// Example: latitude=40.7128, longitude=-74.0060 → "40_-74"
    fn get_tile_id(&self, latitude: f64, longitude: f64) -> String {
        let lat = latitude.floor() as i32;
        let lon = longitude.floor() as i32;
        format!("{}_{}", lat, lon)
    }

    /// Load elevation value from GeoTIFF tile
    /// This is a simplified version - real implementation would:
    /// - Parse GeoTIFF geotransform
    /// - Handle multi-band rasters
    /// - Interpolate between pixels
    fn load_elevation_from_tile(
        &self,
        latitude: f64,
        longitude: f64,
        tile_id: &str,
    ) -> Result<f32> {
        // In production, this would:
        // 1. Load GeoTIFF from data source
        // 2. Parse geotransform metadata
        // 3. Calculate pixel coordinates from lat/lon
        // 4. Read pixel value and interpolate
        //
        // For now, return a placeholder that would be replaced with real GDAL integration

        // Get tile data
        let _tile_data = self.loader.get_tile(tile_id)?;

        // TODO: Implement GeoTIFF parsing with gdal-sys
        // For now, return plausible elevation based on coordinates
        // This would be replaced with real raster processing
        let elevation = self.estimate_elevation_from_coordinates(latitude, longitude);
        Ok(elevation)
    }

    /// Placeholder elevation estimation
    /// In production: read from actual GeoTIFF
    fn estimate_elevation_from_coordinates(&self, latitude: f64, longitude: f64) -> f32 {
        // Rough model: higher latitudes, higher elevations (for northern hemisphere mountains)
        // This is just a placeholder - real implementation reads GeoTIFF
        let base = 100.0;
        let lat_effect = (latitude.abs() - 30.0) * 50.0; // Mountains at higher latitudes
        let lon_effect = ((longitude + 120.0) % 30.0 - 15.0).abs() * 20.0; // Some variation by longitude

        (base + lat_effect + lon_effect).max(0.0)
    }

    /// Classify elevation into category
    pub fn classify_elevation(&self, elevation_m: f32) -> String {
        match elevation_m {
            elev if elev < 100.0 => "LOWLAND".to_string(),
            elev if elev < 500.0 => "HILLS".to_string(),
            elev if elev < 1500.0 => "MOUNTAINS".to_string(),
            elev if elev < 2500.0 => "HIGH_MOUNTAINS".to_string(),
            _ => "ALPINE".to_string(),
        }
    }

    /// Get elevation profile for a region (all tiles)
    pub fn get_elevation_profile(&self, min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> Result<Vec<(f64, f64, f32)>> {
        let mut profile = Vec::new();

        let lat_min = min_lat.floor() as i32;
        let lat_max = max_lat.floor() as i32;
        let lon_min = min_lon.floor() as i32;
        let lon_max = max_lon.floor() as i32;

        for lat in lat_min..=lat_max {
            for lon in lon_min..=lon_max {
                let elevation = self.get_elevation(lat as f64, lon as f64)?;
                profile.push((lat as f64, lon as f64, elevation.elevation_m));
            }
        }

        Ok(profile)
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
    fn test_lapse_rate() {
        let service = ElevationService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                std::path::PathBuf::from("/tmp"),
                "test.tif".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(1).unwrap()),
            )),
        };

        // At 1000m elevation
        let adjustment = service.calculate_lapse_rate(1000.0);
        assert!((adjustment - (-6.5)).abs() < 0.01); // -0.65°C/100m * 10 = -6.5°C
    }

    #[test]
    fn test_elevation_classification() {
        let service = ElevationService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                std::path::PathBuf::from("/tmp"),
                "test.tif".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(1).unwrap()),
            )),
        };

        assert_eq!(service.classify_elevation(50.0), "LOWLAND");
        assert_eq!(service.classify_elevation(500.0), "MOUNTAINS");
        assert_eq!(service.classify_elevation(2000.0), "HIGH_MOUNTAINS");
    }
}
