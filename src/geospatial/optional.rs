//! Optional geospatial layers backed by user-supplied GeoJSON tiles.
//!
//! These layers are opt-in: configure a data source per layer and provide a
//! GeoJSON `FeatureCollection` per 1° tile (the same tile ids as elevation/UHI).
//! Values are read from feature properties — no values are invented when the
//! source data does not contain them. Missing tiles or missing required
//! properties return a real error.
//!
//! Property contracts:
//! - vegetation: `ndvi` (number), `vegetation_type` (string), optional
//!   `cooling_effect_c` (number, defaults to 0.0 when absent)
//! - soil: `texture` (string), `water_holding_capacity_mm` (number), `ph` (number)
//! - flood risk: `risk_score` (number), `risk_category` (string), optional
//!   `factors` (object of numbers)

use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;

use crate::geospatial::config::{DataSourceConfig, GeospatialConfig};
use crate::geospatial::data_source::{create_loader, GeoDataLoader};
use crate::geospatial::{FloodRiskData, SoilData, VegetationData};

fn tile_id(latitude: f64, longitude: f64) -> String {
    format!("{}_{}", latitude.floor() as i32, longitude.floor() as i32)
}

fn load_features(
    loader: &Arc<dyn GeoDataLoader>,
    latitude: f64,
    longitude: f64,
) -> Result<Vec<Value>> {
    let tile = tile_id(latitude, longitude);
    let raw = loader.get_vector(&tile)?;
    let geojson: Value = serde_json::from_str(&raw)?;
    let features = geojson
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("tile {tile} is not a GeoJSON FeatureCollection"))?;
    Ok(features.clone())
}

fn first_feature(features: &[Value]) -> Result<&Value> {
    features
        .first()
        .ok_or_else(|| anyhow::anyhow!("GeoJSON tile contains no features"))
}

fn number_property(feature: &Value, name: &str) -> Result<f64> {
    feature
        .get("properties")
        .and_then(|props| props.get(name))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("feature is missing numeric property '{name}'"))
}

fn string_property(feature: &Value, name: &str) -> Result<String> {
    feature
        .get("properties")
        .and_then(|props| props.get(name))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("feature is missing string property '{name}'"))
}

/// Vegetation/NDVI layer (optional, GeoJSON-backed).
pub struct VegetationService {
    loader: Arc<dyn GeoDataLoader>,
}

impl VegetationService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        Ok(Self {
            loader: create_loader(&config.source)?,
        })
    }

    pub fn get_data(&self, latitude: f64, longitude: f64) -> Result<VegetationData> {
        let features = load_features(&self.loader, latitude, longitude)?;
        let feature = first_feature(&features)?;
        let ndvi = number_property(feature, "ndvi")? as f32;
        if !(0.0..=1.0).contains(&ndvi) {
            anyhow::bail!("ndvi must be between 0 and 1, got {ndvi}");
        }
        let vegetation_type = string_property(feature, "vegetation_type")?;
        let cooling_effect_c = feature
            .get("properties")
            .and_then(|props| props.get("cooling_effect_c"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        Ok(VegetationData {
            ndvi,
            vegetation_type,
            cooling_effect_c,
        })
    }
}

/// Soil layer (optional, GeoJSON-backed).
pub struct SoilService {
    loader: Arc<dyn GeoDataLoader>,
}

impl SoilService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        Ok(Self {
            loader: create_loader(&config.source)?,
        })
    }

    pub fn get_data(&self, latitude: f64, longitude: f64) -> Result<SoilData> {
        let features = load_features(&self.loader, latitude, longitude)?;
        let feature = first_feature(&features)?;
        Ok(SoilData {
            texture: string_property(feature, "texture")?,
            water_holding_capacity_mm: number_property(feature, "water_holding_capacity_mm")?
                as f32,
            ph: number_property(feature, "ph")? as f32,
        })
    }
}

/// Flood risk layer (optional, GeoJSON-backed).
pub struct FloodRiskService {
    loader: Arc<dyn GeoDataLoader>,
}

impl FloodRiskService {
    pub fn new(config: &DataSourceConfig) -> Result<Self> {
        Ok(Self {
            loader: create_loader(&config.source)?,
        })
    }

    pub fn get_data(&self, latitude: f64, longitude: f64) -> Result<FloodRiskData> {
        let features = load_features(&self.loader, latitude, longitude)?;
        let feature = first_feature(&features)?;
        let risk_score = number_property(feature, "risk_score")? as f32;
        let risk_category = string_property(feature, "risk_category")?;
        let mut factors: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        if let Some(object) = feature
            .get("properties")
            .and_then(|props| props.get("factors"))
            .and_then(Value::as_object)
        {
            for (key, value) in object {
                if let Some(number) = value.as_f64() {
                    factors.insert(key.clone(), number as f32);
                }
            }
        }
        Ok(FloodRiskData {
            risk_score,
            risk_category,
            factors,
        })
    }
}

/// Google Maps Reverse Geocoding (optional; not implemented).
pub struct GoogleMapsReverseGeocoder {
    _api_key: String,
}

impl GoogleMapsReverseGeocoder {
    pub fn new(api_key: &str) -> Result<Self> {
        Ok(Self {
            _api_key: api_key.to_string(),
        })
    }

    pub fn reverse_geocode(
        &self,
        _latitude: f64,
        _longitude: f64,
    ) -> Result<super::reverse_geocoding::ReverseGeocodeResult> {
        anyhow::bail!(
            "Google Maps reverse geocoding is not implemented; use the Nominatim/OSM service"
        )
    }
}

/// USPS Postal Database (optional; not implemented).
pub struct USPSPostalDatabase {
    _db_path: String,
}

impl USPSPostalDatabase {
    pub fn new(db_path: &str) -> Result<Self> {
        Ok(Self {
            _db_path: db_path.to_string(),
        })
    }

    pub fn reverse_geocode(
        &self,
        _latitude: f64,
        _longitude: f64,
    ) -> Result<super::reverse_geocoding::ReverseGeocodeResult> {
        anyhow::bail!(
            "USPS postal database lookup is not implemented; use the Nominatim/OSM service"
        )
    }
}

/// Container for all optional services.
pub struct OptionalServices {
    pub vegetation: Option<VegetationService>,
    pub soil: Option<SoilService>,
    pub flood_risk: Option<FloodRiskService>,
    pub google_maps_geocoder: Option<GoogleMapsReverseGeocoder>,
    pub usps_database: Option<USPSPostalDatabase>,
}

impl OptionalServices {
    pub fn new(config: &GeospatialConfig) -> Result<Self> {
        let vegetation = config
            .vegetation
            .as_ref()
            .map(VegetationService::new)
            .transpose()?;
        let soil = config.soil.as_ref().map(SoilService::new).transpose()?;
        let flood_risk = config
            .flood_risk
            .as_ref()
            .map(FloodRiskService::new)
            .transpose()?;
        Ok(OptionalServices {
            vegetation,
            soil,
            flood_risk,
            google_maps_geocoder: None,
            usps_database: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geospatial::config::{DataSource, DataSourceConfig};

    fn local_config(dir: &std::path::Path, pattern: &str) -> DataSourceConfig {
        DataSourceConfig {
            source: DataSource::LocalFile {
                base_path: dir.to_path_buf(),
                file_pattern: pattern.to_string(),
            },
            cache_ttl_seconds: 60,
            timeout_seconds: 5,
        }
    }

    fn write_tile(dir: &std::path::Path, name: &str, properties: &str) {
        std::fs::write(
            dir.join(name),
            format!(
                r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","properties":{properties},"geometry":{{"type":"Point","coordinates":[0,0]}}}}]}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn vegetation_reads_real_properties() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(
            dir.path(),
            "veg_40_-74.geojson",
            r#"{"ndvi":0.72,"vegetation_type":"forest","cooling_effect_c":2.1}"#,
        );
        let service =
            VegetationService::new(&local_config(dir.path(), "veg_{lat}_{lon}.geojson")).unwrap();
        let data = service.get_data(40.7, -74.0).unwrap();
        assert!((data.ndvi - 0.72).abs() < 1e-6);
        assert_eq!(data.vegetation_type, "forest");
        assert!((data.cooling_effect_c - 2.1).abs() < 1e-6);
    }

    #[test]
    fn soil_and_flood_read_real_properties() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(
            dir.path(),
            "soil_40_-74.geojson",
            r#"{"texture":"loam","water_holding_capacity_mm":180.0,"ph":6.5}"#,
        );
        write_tile(
            dir.path(),
            "flood_40_-74.geojson",
            r#"{"risk_score":0.42,"risk_category":"moderate","factors":{"rainfall":0.4}}"#,
        );
        let soil = SoilService::new(&local_config(dir.path(), "soil_{lat}_{lon}.geojson"))
            .unwrap()
            .get_data(40.7, -74.0)
            .unwrap();
        assert_eq!(soil.texture, "loam");
        assert!((soil.water_holding_capacity_mm - 180.0).abs() < 1e-6);
        assert!((soil.ph - 6.5).abs() < 1e-6);
    }

    #[test]
    fn missing_property_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(dir.path(), "veg_40_-74.geojson", r#"{"ndvi":0.5}"#);
        let service =
            VegetationService::new(&local_config(dir.path(), "veg_{lat}_{lon}.geojson")).unwrap();
        assert!(service.get_data(40.7, -74.0).is_err());
    }
}
