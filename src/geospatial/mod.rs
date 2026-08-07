/// Geospatial module for weather enrichment
///
/// CRITICAL (always available):
/// - Elevation (SRTM): Temperature lapse rate, wind adjustment
/// - Urban Heat Island (OSM): Building density, UHI effect
///
/// OPTIONAL (framework stubs, load on demand):
/// - Vegetation (NDVI): Cooling effects, drought detection
/// - Soil: Water holding capacity, irrigation
/// - Flood Risk: Hazard modeling, early warning

pub mod config;
pub mod elevation;
pub mod urban_heat_island;
pub mod reverse_geocoding;
pub mod optional;
pub mod data_source;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use config::GeospatialConfig;
pub use data_source::DataSource;

/// Result from geospatial enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeospatialContext {
    // Critical: Always available
    pub elevation: ElevationData,
    pub urban_heat_island: UHIData,

    // Optional: Only if requested
    pub vegetation: Option<VegetationData>,
    pub soil: Option<SoilData>,
    pub flood_risk: Option<FloodRiskData>,
}

/// Elevation data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElevationData {
    pub elevation_m: f32,
    pub lapse_rate_adjustment_c: f32,
    pub terrain_roughness: f32,
}

/// Urban Heat Island data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UHIData {
    pub building_density_percent: f32,
    pub average_building_height_m: f32,
    pub uhi_effect_c: f32,
    pub location_type: String, // "urban", "suburban", "rural"
}

/// Vegetation data (optional)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VegetationData {
    pub ndvi: f32,
    pub vegetation_type: String,
    pub cooling_effect_c: f32,
}

/// Soil data (optional)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoilData {
    pub texture: String,
    pub water_holding_capacity_mm: f32,
    pub ph: f32,
}

/// Flood risk data (optional)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloodRiskData {
    pub risk_score: f32,
    pub risk_category: String,
    pub factors: HashMap<String, f32>,
}

/// Main geospatial enricher
pub struct GeospatialEnricher {
    config: GeospatialConfig,
    elevation_service: elevation::ElevationService,
    uhi_service: urban_heat_island::UHIService,
    optional_services: optional::OptionalServices,
}

impl GeospatialEnricher {
    pub fn new(config: GeospatialConfig) -> Result<Self> {
        let elevation_service = elevation::ElevationService::new(&config.elevation)?;
        let uhi_service = urban_heat_island::UHIService::new(&config.urban_heat_island)?;
        let optional_services = optional::OptionalServices::new(&config)?;

        Ok(GeospatialEnricher {
            config,
            elevation_service,
            uhi_service,
            optional_services,
        })
    }

    /// Enrich location with geospatial context
    ///
    /// Critical layers (elevation, UHI) are always loaded
    /// Optional layers only loaded if requested
    pub fn enrich(
        &self,
        latitude: f64,
        longitude: f64,
        requested_layers: &[&str],
    ) -> Result<GeospatialContext> {
        // Critical: Always compute
        let elevation = self.elevation_service.get_elevation(latitude, longitude)?;
        let uhi = self.uhi_service.get_uhi_data(latitude, longitude)?;

        // Optional: Only if requested
        let vegetation = if requested_layers.contains(&"vegetation") {
            Some(self.optional_services.vegetation.get_data(latitude, longitude)?)
        } else {
            None
        };

        let soil = if requested_layers.contains(&"soil") {
            Some(self.optional_services.soil.get_data(latitude, longitude)?)
        } else {
            None
        };

        let flood_risk = if requested_layers.contains(&"flood_risk") {
            Some(self.optional_services.flood_risk.get_data(latitude, longitude)?)
        } else {
            None
        };

        Ok(GeospatialContext {
            elevation,
            urban_heat_island: uhi,
            vegetation,
            soil,
            flood_risk,
        })
    }
}
