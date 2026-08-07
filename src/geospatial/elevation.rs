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

    /// Load the real elevation value from a GeoTIFF tile: parses the
    /// ModelPixelScaleTag/ModelTiepointTag geo-referencing tags (standard
    /// GeoTIFF, e.g. SRTM 30m COG products) to compute the affine
    /// transform, maps (lat, lon) to a raster pixel, and reads that pixel's
    /// real value from the decoded band. No interpolation between pixels
    /// (nearest-pixel sample) — sub-pixel bilinear interpolation is a
    /// possible future refinement, not implemented here.
    fn load_elevation_from_tile(
        &self,
        latitude: f64,
        longitude: f64,
        tile_id: &str,
    ) -> Result<f32> {
        let tile_data = self.loader.get_tile(tile_id)?;
        let cursor = std::io::Cursor::new(tile_data);
        let mut decoder = tiff::decoder::Decoder::new(cursor)
            .map_err(|e| anyhow::anyhow!("failed to parse GeoTIFF for tile {tile_id}: {e}"))?;

        let (width, height) = decoder
            .dimensions()
            .map_err(|e| anyhow::anyhow!("failed to read GeoTIFF dimensions for tile {tile_id}: {e}"))?;

        let pixel_scale = decoder
            .get_tag_f64_vec(tiff::tags::Tag::ModelPixelScaleTag)
            .map_err(|e| anyhow::anyhow!("tile {tile_id} is missing ModelPixelScaleTag (not a georeferenced GeoTIFF?): {e}"))?;
        let tiepoint = decoder
            .get_tag_f64_vec(tiff::tags::Tag::ModelTiepointTag)
            .map_err(|e| anyhow::anyhow!("tile {tile_id} is missing ModelTiepointTag (not a georeferenced GeoTIFF?): {e}"))?;

        if pixel_scale.len() < 2 || tiepoint.len() < 6 {
            return Err(anyhow::anyhow!(
                "tile {tile_id} has malformed geo-referencing tags"
            ));
        }

        // ModelPixelScaleTag = [scaleX, scaleY, scaleZ]
        // ModelTiepointTag   = [rasterI, rasterJ, rasterK, geoX, geoY, geoZ]
        // Raster pixel (rasterI, rasterJ) corresponds to geo coordinate (geoX, geoY).
        let (scale_x, scale_y) = (pixel_scale[0], pixel_scale[1]);
        let (tie_i, tie_j, geo_x, geo_y) = (tiepoint[0], tiepoint[1], tiepoint[3], tiepoint[4]);

        if scale_x == 0.0 || scale_y == 0.0 {
            return Err(anyhow::anyhow!("tile {tile_id} has zero pixel scale"));
        }

        // Raster row increases southward while latitude decreases southward,
        // hence the sign flip on the row computation.
        let col = ((longitude - geo_x) / scale_x + tie_i).round();
        let row = ((geo_y - latitude) / scale_y + tie_j).round();

        if col < 0.0 || row < 0.0 || col >= f64::from(width) || row >= f64::from(height) {
            return Err(anyhow::anyhow!(
                "coordinate ({latitude}, {longitude}) falls outside tile {tile_id}'s raster bounds"
            ));
        }
        let (col, row) = (col as usize, row as usize);

        let image = decoder
            .read_image()
            .map_err(|e| anyhow::anyhow!("failed to decode GeoTIFF raster for tile {tile_id}: {e}"))?;
        let idx = row * (width as usize) + col;

        let raw_value: f64 = match image {
            tiff::decoder::DecodingResult::I16(data) => *data
                .get(idx)
                .ok_or_else(|| anyhow::anyhow!("pixel index out of range for tile {tile_id}"))?
                as f64,
            tiff::decoder::DecodingResult::F32(data) => f64::from(
                *data
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("pixel index out of range for tile {tile_id}"))?,
            ),
            tiff::decoder::DecodingResult::F64(data) => *data
                .get(idx)
                .ok_or_else(|| anyhow::anyhow!("pixel index out of range for tile {tile_id}"))?,
            tiff::decoder::DecodingResult::U16(data) => f64::from(
                *data
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("pixel index out of range for tile {tile_id}"))?,
            ),
            tiff::decoder::DecodingResult::I32(data) => f64::from(
                *data
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("pixel index out of range for tile {tile_id}"))?,
            ),
            other => {
                return Err(anyhow::anyhow!(
                    "unsupported GeoTIFF sample format in tile {tile_id}: {other:?}"
                ))
            }
        };

        // SRTM's standard "void" (no-data) sentinel for Int16 tiles.
        if raw_value <= -32768.0 {
            return Err(anyhow::anyhow!(
                "tile {tile_id} has no elevation data (void/no-data pixel) at ({latitude}, {longitude})"
            ));
        }

        Ok(raw_value as f32)
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

    /// Writes a real, minimal GeoTIFF: a 4x4 single-band Float32 raster
    /// covering (39-40N, -75--74W) at 0.25-degree pixel spacing, with a
    /// known, distinct value in every cell. Used to verify
    /// `load_elevation_from_tile` genuinely reads real geo-referenced pixel
    /// data rather than returning a formula-based guess.
    fn write_test_geotiff(path: &std::path::Path) {
        use tiff::encoder::{colortype::Gray32Float, TiffEncoder};
        use tiff::tags::Tag;

        let file = std::fs::File::create(path).unwrap();
        let mut tiff = TiffEncoder::new(file).unwrap();
        let mut image = tiff.new_image::<Gray32Float>(4, 4).unwrap();

        // Origin at (lon=-75, lat=40), 0.25 deg/pixel, north-up (scaleY
        // positive; row increases southward per the GeoTIFF convention).
        image
            .encoder()
            .write_tag(Tag::ModelPixelScaleTag, &[0.25_f64, 0.25_f64, 0.0_f64][..])
            .unwrap();
        image
            .encoder()
            .write_tag(
                Tag::ModelTiepointTag,
                &[0.0_f64, 0.0_f64, 0.0_f64, -75.0_f64, 40.0_f64, 0.0_f64][..],
            )
            .unwrap();

        // Row-major pixel values: row*100 + col, so each cell is
        // unambiguously identifiable.
        let mut pixels = Vec::with_capacity(16);
        for row in 0..4 {
            for col in 0..4 {
                pixels.push((row * 100 + col) as f32);
            }
        }
        image.write_data(&pixels).unwrap();
    }

    fn elevation_service_for(dir: &std::path::Path) -> ElevationService {
        ElevationService {
            loader: std::sync::Arc::new(super::super::data_source::LocalFileLoader::new(
                dir.to_path_buf(),
                "{lat}_{lon}.tif".to_string(),
            )),
            cache: Arc::new(std::sync::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(100).unwrap()),
            )),
        }
    }

    #[test]
    fn test_load_elevation_from_tile_reads_real_geotiff_pixel() {
        let dir = tempfile::tempdir().unwrap();
        write_test_geotiff(&dir.path().join("39_-75.tif"));
        let service = elevation_service_for(dir.path());

        // Pixel (0,0) covers lon in [-75, -74.75), lat in (39.75, 40] -> value 0.
        let elevation = service
            .load_elevation_from_tile(39.9, -74.9, "39_-75")
            .unwrap();
        assert!((elevation - 0.0).abs() < 0.01);

        // Pixel (row=2, col=3): lat ~39.4, lon ~-74.15 -> value 203.
        let elevation = service
            .load_elevation_from_tile(39.4, -74.15, "39_-75")
            .unwrap();
        assert!((elevation - 203.0).abs() < 0.01);
    }

    #[test]
    fn test_get_elevation_end_to_end_uses_real_pixel_not_formula() {
        let dir = tempfile::tempdir().unwrap();
        write_test_geotiff(&dir.path().join("39_-75.tif"));
        let service = elevation_service_for(dir.path());

        let data = service.get_elevation(39.9, -74.9).unwrap();
        assert!((data.elevation_m - 0.0).abs() < 0.01);
        // Real lapse rate derived from the real (not fabricated) elevation.
        assert!((data.lapse_rate_adjustment_c - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_load_elevation_out_of_bounds_coordinate_is_a_real_error() {
        let dir = tempfile::tempdir().unwrap();
        write_test_geotiff(&dir.path().join("39_-75.tif"));
        let service = elevation_service_for(dir.path());

        // Latitude 10 is nowhere near this tile's 39-40N coverage.
        let result = service.load_elevation_from_tile(10.0, -74.9, "39_-75");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_elevation_missing_tile_is_a_real_error_not_fake_data() {
        let dir = tempfile::tempdir().unwrap();
        let service = elevation_service_for(dir.path());

        let result = service.load_elevation_from_tile(39.9, -74.9, "39_-75");
        assert!(result.is_err());
    }
}
