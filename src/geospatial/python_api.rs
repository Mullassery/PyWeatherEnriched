//! Python bindings for the real (data-backed) geospatial services.
//!
//! Both classes require an explicit local data directory. They do not read
//! hidden global config and they do not fabricate results: a missing tile
//! or a tile without usable data raises an error naming the file/feature.
//!
//! The deliberately unimplemented optional layers (vegetation/NDVI, soil,
//! flood risk) and alternative reverse-geocoding providers are NOT exposed.

use crate::geospatial::config::{DataSource, DataSourceConfig};
use crate::geospatial::reverse_geocoding::ReverseGeocodingService;
use crate::geospatial::urban_heat_island::UHIService;
use pyo3::exceptions::{PyFileNotFoundError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn local_config(base_path: String, file_pattern: String) -> DataSourceConfig {
    DataSourceConfig {
        source: DataSource::LocalFile {
            base_path: base_path.into(),
            file_pattern,
        },
        cache_ttl_seconds: 86400 * 30,
        timeout_seconds: 10,
    }
}

fn validate_coordinates(latitude: f64, longitude: f64) -> Result<(), String> {
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err(format!(
            "latitude must be a finite value in [-90, 90], got {latitude}"
        ));
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err(format!(
            "longitude must be a finite value in [-180, 180], got {longitude}"
        ));
    }
    Ok(())
}

fn data_err(error: anyhow::Error) -> PyErr {
    let message = error.to_string();
    if message.contains("No such file or directory") {
        PyFileNotFoundError::new_err(message)
    } else {
        PyRuntimeError::new_err(message)
    }
}

fn validation_err(message: String) -> PyErr {
    PyValueError::new_err(message)
}

/// Reverse geocode coordinates using a local OSM GeoJSON tile directory.
///
/// Tiles are read from ``base_path`` using ``file_pattern`` where ``{lat}``
/// and ``{lon}`` are replaced by ``floor(latitude)`` / ``floor(longitude)``
/// (e.g. ``40_-74.geojson``). Raises ``FileNotFoundError`` when the tile is
/// absent and ``RuntimeError`` when it contains no usable feature.
#[pyclass(name = "ReverseGeocoder")]
pub struct PyReverseGeocoder {
    service: ReverseGeocodingService,
}

#[pymethods]
impl PyReverseGeocoder {
    #[new]
    #[pyo3(signature = (base_path, file_pattern = "{lat}_{lon}.geojson".to_string(), cache_enabled = true))]
    fn new(base_path: String, file_pattern: String, cache_enabled: bool) -> PyResult<Self> {
        let config = local_config(base_path, file_pattern);
        let service = ReverseGeocodingService::new(&config, cache_enabled).map_err(data_err)?;
        Ok(PyReverseGeocoder { service })
    }

    /// Return the nearest OSM feature's real address fields as a dict.
    fn reverse_geocode<'py>(
        &self,
        py: Python<'py>,
        latitude: f64,
        longitude: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        validate_coordinates(latitude, longitude).map_err(validation_err)?;
        let result = self
            .service
            .reverse_geocode(latitude, longitude)
            .map_err(data_err)?;

        let out = PyDict::new(py);
        out.set_item("postal_code", result.postal_code)?;
        out.set_item("postal_code_type", result.postal_code_type)?;
        out.set_item("street_address", result.street_address)?;
        out.set_item("city", result.city)?;
        out.set_item("state", result.state)?;
        out.set_item("country", result.country)?;
        out.set_item("country_code", result.country_code)?;
        out.set_item("admin_level_1", result.admin_level_1)?;
        out.set_item("admin_level_2", result.admin_level_2)?;
        out.set_item("neighborhood", result.neighborhood)?;
        out.set_item("latitude", result.latitude)?;
        out.set_item("longitude", result.longitude)?;
        out.set_item("confidence", result.confidence)?;
        out.set_item("source", result.source)?;
        Ok(out)
    }

    fn clear_cache(&self) {
        self.service.clear_cache();
    }

    /// ``(entries, capacity)`` for the in-memory LRU cache.
    fn cache_stats(&self) -> (usize, usize) {
        self.service.cache_stats()
    }
}

/// Estimate urban-heat-island metrics from local OSM building footprints.
///
/// This is a model estimate computed from real OSM building density and
/// heights (see `urban_heat_island.rs`), not an observed temperature. Uses
/// the same ``{lat}_{lon}.geojson`` tile lookup as `ReverseGeocoder`.
#[pyclass(name = "UrbanHeatIsland")]
pub struct PyUrbanHeatIsland {
    service: UHIService,
}

#[pymethods]
impl PyUrbanHeatIsland {
    #[new]
    #[pyo3(signature = (base_path, file_pattern = "{lat}_{lon}.geojson".to_string()))]
    fn new(base_path: String, file_pattern: String) -> PyResult<Self> {
        let config = local_config(base_path, file_pattern);
        let service = UHIService::new(&config).map_err(data_err)?;
        Ok(PyUrbanHeatIsland { service })
    }

    /// Return building density, average height, UHI estimate, and class.
    fn get_uhi_data<'py>(
        &self,
        py: Python<'py>,
        latitude: f64,
        longitude: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        validate_coordinates(latitude, longitude).map_err(validation_err)?;
        let result = self
            .service
            .get_uhi_data(latitude, longitude)
            .map_err(data_err)?;

        let out = PyDict::new(py);
        out.set_item("building_density_percent", result.building_density_percent)?;
        out.set_item(
            "average_building_height_m",
            result.average_building_height_m,
        )?;
        out.set_item("uhi_effect_c", result.uhi_effect_c)?;
        out.set_item("location_type", result.location_type)?;
        Ok(out)
    }

    fn clear_cache(&self) {
        self.service.clear_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tile(dir: &std::path::Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn validate_coordinates_accepts_real_ranges_and_rejects_invalid() {
        assert!(validate_coordinates(40.7484, -73.9857).is_ok());
        assert!(validate_coordinates(0.0, 0.0).is_ok());
        assert!(validate_coordinates(90.0, 180.0).is_ok());
        assert!(validate_coordinates(-90.0, -180.0).is_ok());

        assert!(validate_coordinates(91.0, 0.0).is_err());
        assert!(validate_coordinates(0.0, 181.0).is_err());
        assert!(validate_coordinates(f64::NAN, 0.0).is_err());
        assert!(validate_coordinates(0.0, f64::INFINITY).is_err());
    }

    #[test]
    fn missing_tile_is_a_real_error_not_a_fabricated_address() {
        let dir = tempfile::tempdir().unwrap();
        let geocoder = PyReverseGeocoder::new(
            dir.path().to_string_lossy().into_owned(),
            "{lat}_{lon}.geojson".to_string(),
            true,
        )
        .unwrap();

        let error = geocoder
            .service
            .reverse_geocode(40.0, -74.0)
            .expect_err("missing tile must fail");

        assert!(error.to_string().contains("40_-74.geojson"));
        assert!(error.to_string().contains("No such file or directory"));
        assert_eq!(geocoder.cache_stats(), (0, 5000));
    }

    #[test]
    fn reverse_geocode_reads_real_fixture_properties() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(
            dir.path(),
            "40_-74.geojson",
            r#"{
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "geometry": {"type": "Point", "coordinates": [-74.006, 40.7128]},
                        "properties": {
                            "addr:postcode": "10001",
                            "addr:city": "New York",
                            "addr:state": "NY",
                            "addr:country": "US"
                        }
                    }
                ]
            }"#,
        );

        let geocoder = PyReverseGeocoder::new(
            dir.path().to_string_lossy().into_owned(),
            "{lat}_{lon}.geojson".to_string(),
            true,
        )
        .unwrap();
        let result = geocoder.service.reverse_geocode(40.7128, -74.0).unwrap();

        assert_eq!(result.postal_code, "10001");
        assert_eq!(result.city, Some("New York".to_string()));
        assert_eq!(result.country_code, "US");
        assert_eq!(geocoder.cache_stats(), (1, 5000));
    }

    #[test]
    fn uhi_reads_real_building_footprints() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(
            dir.path(),
            "40_-74.geojson",
            r#"{
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [[
                                [-74.001, 40.001], [-74.0, 40.001],
                                [-74.0, 40.0], [-74.001, 40.0], [-74.001, 40.001]
                            ]]
                        },
                        "properties": {"building": "yes", "height": "20"}
                    }
                ]
            }"#,
        );

        let uhi = PyUrbanHeatIsland::new(
            dir.path().to_string_lossy().into_owned(),
            "{lat}_{lon}.geojson".to_string(),
        )
        .unwrap();
        let result = uhi.service.get_uhi_data(40.0005, -74.0).unwrap();

        // ~111m x ~85m square at 40N => ~9,500 m^2 of a 1km^2 search area.
        assert!((result.building_density_percent - 0.95).abs() < 0.1);
        assert!((result.average_building_height_m - 20.0).abs() < 0.01);
        assert!(result.uhi_effect_c > 0.7);
        assert_eq!(result.location_type, "rural");
    }

    #[test]
    fn uhi_missing_tile_is_a_real_error() {
        let dir = tempfile::tempdir().unwrap();
        let uhi = PyUrbanHeatIsland::new(
            dir.path().to_string_lossy().into_owned(),
            "{lat}_{lon}.geojson".to_string(),
        )
        .unwrap();

        let error = uhi
            .service
            .get_uhi_data(40.0, -74.0)
            .expect_err("missing tile must fail");
        assert!(error.to_string().contains("No such file or directory"));
    }
}
