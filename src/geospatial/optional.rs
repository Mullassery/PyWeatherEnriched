/// Optional geospatial layers - framework stubs
///
/// These are NOT implemented by default. Users must opt-in via config.
/// Structure is provided for easy extension.

use crate::geospatial::config::GeospatialConfig;
use crate::geospatial::{SoilData, VegetationData, FloodRiskData};
use anyhow::Result;

/// Vegetation/NDVI layer (optional)
pub struct VegetationService {
    // TODO: Implement when needed
    // - Sentinel-2 NDVI raster processing
    // - Vegetation type classification
    // - Cooling effect calculation
}

impl VegetationService {
    pub fn new(_config: &GeospatialConfig) -> Result<Self> {
        // Load Sentinel-2 NDVI data source
        Ok(VegetationService {})
    }

    pub fn get_data(&self, _latitude: f64, _longitude: f64) -> Result<VegetationData> {
        // TODO: Load NDVI tile and extract value
        Err(anyhow::anyhow!("Vegetation module not yet implemented"))
    }
}

/// Soil data layer (optional)
pub struct SoilService {
    // TODO: Implement when needed
    // - SoilGrids/HWSD raster processing
    // - Soil texture classification (sand/clay/silt %)
    // - Water holding capacity calculation
}

impl SoilService {
    pub fn new(_config: &GeospatialConfig) -> Result<Self> {
        // Load soil data source
        Ok(SoilService {})
    }

    pub fn get_data(&self, _latitude: f64, _longitude: f64) -> Result<SoilData> {
        // TODO: Load soil tile and extract properties
        Err(anyhow::anyhow!("Soil module not yet implemented"))
    }
}

/// Flood risk layer (optional)
pub struct FloodRiskService {
    // TODO: Implement when needed
    // - DEM slope analysis
    // - Rainfall frequency
    // - Land use vulnerability
}

impl FloodRiskService {
    pub fn new(_config: &GeospatialConfig) -> Result<Self> {
        Ok(FloodRiskService {})
    }

    pub fn get_data(&self, _latitude: f64, _longitude: f64) -> Result<FloodRiskData> {
        // TODO: Calculate flood risk from multiple factors
        Err(anyhow::anyhow!("Flood risk module not yet implemented"))
    }
}

/// Container for all optional services
pub struct OptionalServices {
    pub vegetation: VegetationService,
    pub soil: SoilService,
    pub flood_risk: FloodRiskService,
}

impl OptionalServices {
    pub fn new(config: &GeospatialConfig) -> Result<Self> {
        // Load optional services only if configured
        // Otherwise, services will return "not implemented" errors

        let vegetation = VegetationService::new(config)
            .unwrap_or_else(|_| VegetationService {});

        let soil = SoilService::new(config)
            .unwrap_or_else(|_| SoilService {});

        let flood_risk = FloodRiskService::new(config)
            .unwrap_or_else(|_| FloodRiskService {});

        Ok(OptionalServices {
            vegetation,
            soil,
            flood_risk,
        })
    }
}
