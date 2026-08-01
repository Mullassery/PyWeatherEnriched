# Geospatial Integration - Technical Roadmap & Stack

Detailed implementation plan for adding geospatial data layers to PyWeatherEnriched.

---

## Technology Stack Overview

### Core Geospatial Libraries

| Library | Language | Use Case | Size | Speed |
|---------|----------|----------|------|-------|
| **GDAL/GEOS** | C++ (Rust FFI) | Raster processing, geometry ops | Heavy | Fast |
| **Proj** | C | Coordinate transformations | Small | Very fast |
| **Rasterio** | Python | GeoTIFF reading | Light | Moderate |
| **Shapely** | Python | Vector geometry | Light | Moderate |
| **Osmium** | C++ (Rust FFI) | OSM data parsing | Medium | Very fast |
| **PostGIS** | PostgreSQL extension | Vector spatial queries | Heavy | Moderate |
| **Tile38** | Standalone | Geo-indexing | Light | Very fast |

### Recommended Stack for PyWeatherEnriched

```
┌─────────────────────────────────────────────────────────┐
│                    Python API Layer                     │
│              (PyWeatherEnriched Python 3.10+)           │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│                  PyO3 Rust Bindings                     │
│         (Thread-safe, zero-copy data transfer)          │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Rust Geospatial Engine                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Raster Ops   │  │ Vector Ops   │  │ Index Ops    │  │
│  │ (GDAL)       │  │ (GEOS)       │  │ (Tile38)     │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Data Storage & Caching                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ GeoTIFF      │  │ Redis        │  │ PostGIS      │  │
│  │ (Tiles)      │  │ (Hot cache)  │  │ (Vector DB)  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1: Elevation Data (SRTM) - v0.5.0

### Requirements

**Dependencies to Add**:
```toml
# Cargo.toml
gdal = "0.16"  # GDAL Rust bindings
tiff = "0.9"   # For GeoTIFF parsing
```

**Data to Download**:
- SRTM 30m global DEM (~45 GB compressed, ~500 GB uncompressed)
- Divided into 1°x1° tiles (3600x3600 pixels each)
- Store as GeoTIFF tiles

### Implementation

```rust
// src/geospatial/elevation.rs

use gdal::Dataset;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ElevationService {
    tiles: Arc<HashMap<String, Dataset>>, // Cached tiles
    tile_cache_dir: String,
}

impl ElevationService {
    pub fn new(cache_dir: &str) -> Result<Self> {
        Ok(ElevationService {
            tiles: Arc::new(HashMap::new()),
            tile_cache_dir: cache_dir.to_string(),
        })
    }

    /// Get elevation at a specific coordinate
    pub fn get_elevation(&self, latitude: f64, longitude: f64) -> Result<f32> {
        let tile_id = format!("{}_{}", 
            (latitude as i32), 
            (longitude as i32)
        );
        
        let dataset = self.load_tile(&tile_id)?;
        let band = dataset.rasterband(1)?;
        
        // Convert lat/lon to pixel coordinates
        let geotransform = dataset.geo_transform()?;
        let pixel_x = ((longitude - geotransform[0]) / geotransform[1]) as usize;
        let pixel_y = ((latitude - geotransform[3]) / geotransform[5]) as usize;
        
        // Read elevation value
        let data = band.read_as::<f32>(
            pixel_x as isize,
            pixel_y as isize,
            1,
            1
        )?;
        
        Ok(data[0])
    }

    /// Calculate lapse rate (temperature change with elevation)
    pub fn calculate_lapse_rate_adjustment(
        &self,
        elevation_m: f32,
        base_temperature_c: f32,
        lapse_rate: f32, // typically -0.65°C per 100m
    ) -> f32 {
        (elevation_m / 100.0) * lapse_rate
    }

    /// Estimate wind speed reduction based on elevation (roughness)
    pub fn estimate_wind_reduction(&self, elevation_m: f32) -> f32 {
        // Sheltered valleys (high elevation variation) reduce wind
        // Exposed peaks increase wind
        let roughness = if elevation_m > 2000.0 { 0.8 } else { 1.0 };
        roughness
    }

    fn load_tile(&self, tile_id: &str) -> Result<Dataset> {
        // Implementation: load GeoTIFF from cache_dir or download
    }
}
```

### Data Download & Preprocessing

```python
# Python helper script
import subprocess
import os

class SRTMDownloader:
    def download_tiles(self, lat_range, lon_range, cache_dir):
        """Download SRTM tiles via USGS service"""
        for lat in range(lat_range[0], lat_range[1]):
            for lon in range(lon_range[0], lon_range[1]):
                tile_url = f"https://lpdaac.usgs.gov/appeears/api/.../{lat}_{lon}.tif"
                
                # Download GeoTIFF
                # Convert to COG (Cloud Optimized GeoTIFF) for faster access
                subprocess.run([
                    'gdalwarp',
                    '-of', 'COG',
                    '-co', 'COMPRESS=ZSTD',  # Compression
                    f'{tile_url}',
                    f'{cache_dir}/{lat}_{lon}_COG.tif'
                ])
```

### Caching Strategy

```python
# Redis caching for elevation queries
cache_key = f"elevation:{lat}:{lon}:srtm30m"

# Store at 0.01° precision (roughly 1km × 1km grid)
cached_elevation = redis.get(cache_key)
if cached_elevation:
    return float(cached_elevation)

# Otherwise compute and cache
elevation = srtm_service.get_elevation(lat, lon)
redis.setex(cache_key, ttl=86400*30, value=elevation)  # 30-day TTL

return elevation
```

### Python API

```python
from pyweatherenriched import ElevationService, GeoSpatialEnricher

# Initialize
enricher = GeoSpatialEnricher(cache=distributed_cache)
enricher.add_elevation_data(source='srtm30m', cache_dir='/data/srtm')

# Use in enrichment
enriched = enricher.enrich(
    location='Denver',
    latitude=39.7392,
    longitude=-104.9903,
    timestamp='2024-01-15T12:00:00Z'
)

# Returns:
# {
#   'location': 'Denver',
#   'temperature': 5.0,
#   'elevation_m': 1609,
#   'temperature_lapse_adjusted': 4.5,  # -0.65°C per 100m
#   'elevation_category': 'HIGH_ALTITUDE'
# }
```

### Effort: 2-3 weeks
- Week 1: GDAL setup, tile management, caching
- Week 2: PyO3 bindings, Python API
- Week 3: Testing, documentation, example notebooks

---

## Phase 2: OpenStreetMap Integration - v0.5.0

### Requirements

**Data Format**: Vector data (nodes, ways, relations)
**Size**: ~150 GB uncompressed, ~50 GB compressed
**Query Tool**: Overpass API or OSM extracts

### Implementation

```rust
// src/geospatial/osm.rs

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Building {
    pub id: u64,
    pub height: Option<f32>,
    pub levels: Option<u8>,
    pub material: Option<String>,
    pub area_m2: f32,
}

#[derive(Debug, Clone)]
pub struct LandUse {
    pub type_: String,  // urban, agricultural, forest, etc.
    pub area_m2: f32,
    pub density: f32,   // 0-1 scale
}

pub struct OSMEnricher {
    // Pre-processed OSM data
    buildings_index: SpatialIndex<Building>,
    landuse_index: SpatialIndex<LandUse>,
}

impl OSMEnricher {
    /// Calculate Urban Heat Island effect
    pub fn calculate_uhi(
        &self,
        latitude: f64,
        longitude: f64,
        radius_m: f32,
    ) -> UHIResult {
        // Query buildings within radius
        let buildings = self.buildings_index
            .query_radius(latitude, longitude, radius_m);
        
        let total_building_area: f32 = buildings
            .iter()
            .map(|b| b.area_m2)
            .sum();
        
        let search_area = std::f32::consts::PI * radius_m * radius_m;
        let building_density = total_building_area / search_area;
        
        let avg_height = buildings
            .iter()
            .filter_map(|b| b.height)
            .sum::<f32>() / buildings.len() as f32;
        
        // UHI = 0.7 + 0.25 * building_density + 0.1 * avg_height
        let uhi_celsius = 0.7 + 0.25 * building_density + 0.1 * avg_height;
        
        UHIResult {
            building_density,
            average_building_height: avg_height,
            uhi_effect_celsius: uhi_celsius,
        }
    }

    /// Estimate surface roughness for wind calculations
    pub fn estimate_surface_roughness(
        &self,
        latitude: f64,
        longitude: f64,
        radius_m: f32,
    ) -> f32 {
        let buildings = self.buildings_index
            .query_radius(latitude, longitude, radius_m);
        
        let avg_height = buildings
            .iter()
            .filter_map(|b| b.height)
            .sum::<f32>() / buildings.len() as f32;
        
        // Roughness length = avg_height * 0.05 to 0.1
        // Urban: 1.0-2.0m, Suburban: 0.3-0.5m, Rural: 0.05-0.1m
        match avg_height {
            0.0..=5.0 => 0.1,      // Rural (trees, grass)
            5.0..=15.0 => 0.3,     // Suburban (houses)
            _ => 1.5,              // Urban (tall buildings)
        }
    }

    /// Find nearest water body
    pub fn find_nearest_water(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Option<WaterBody> {
        // Query water index
        // Return: type (ocean, river, lake), distance, effect on climate
    }
}

// Spatial indexing for fast queries
pub struct SpatialIndex<T> {
    // R-tree structure for O(log N) spatial queries
}
```

### Data Import Pipeline

```python
# Download and process OSM data
import osmium
import geopandas as gpd

class OSMProcessor:
    def process_country(self, country_code, output_dir):
        """Download and process OSM for a country"""
        
        # Download from Geofabrik
        url = f"https://download.geofabrik.de/{country_code}-latest.osm.pbf"
        subprocess.run(['wget', url])
        
        # Parse using osmium
        handler = OSMDataHandler(output_dir)
        osmium.SimpleHandler.apply(handler)
        
        # Create spatial indices (R-tree)
        buildings_gdf = gpd.GeoDataFrame.from_file(f'{output_dir}/buildings.geojson')
        buildings_gdf.sindex  # Create spatial index
```

### Caching with Tiles

```python
# Cache OSM data at tile level (e.g., 1° × 1° tiles)
cache_key = f"osm:buildings:{tile_id}"

buildings = redis.get(cache_key)
if buildings:
    return json.loads(buildings)

# Otherwise query and cache
buildings = osm_index.query_tile(tile_id)
redis.setex(cache_key, ttl=86400*30, value=json.dumps(buildings))

return buildings
```

### Effort: 3-4 weeks
- Week 1: Spatial indexing (R-tree), tile management
- Week 2: OSM parser, building/landuse extraction
- Week 3: PyO3 bindings, UHI/wind calculations
- Week 4: Testing, documentation, examples

---

## Phase 3: Sentinel Vegetation Data - v0.5.0

### Requirements

**Data**: Sentinel-2 NDVI (Normalized Difference Vegetation Index)
**Resolution**: 10m (Sentinel-2), free from Copernicus
**Update Frequency**: Every 5-10 days (cloud-dependent)
**Size**: ~500 GB for annual global coverage

### Implementation

```rust
// src/geospatial/vegetation.rs

pub struct VegetationService {
    ndvi_tiles: Arc<HashMap<String, GeoTIFF>>,
    update_schedule: VegetationUpdateSchedule,
}

impl VegetationService {
    /// Get NDVI at specific coordinate and date
    pub fn get_ndvi(
        &self,
        latitude: f64,
        longitude: f64,
        date: DateTime<Utc>,
    ) -> Result<f32> {
        // NDVI = (NIR - RED) / (NIR + RED)
        // Range: -1.0 to 1.0
        // -1.0 to -0.2: Water
        // -0.2 to 0.2: Barren soil/urban
        // 0.2 to 0.5: Sparse vegetation
        // 0.5+: Dense vegetation
        
        let tile_id = self.get_tile_id(latitude, longitude, date);
        let dataset = self.load_ndvi_tile(&tile_id)?;
        
        // Interpolate value at coordinate
        let ndvi = self.bilinear_interpolate(&dataset, latitude, longitude)?;
        
        Ok(ndvi)
    }

    /// Get vegetation trend (improving/degrading)
    pub fn get_ndvi_trend(
        &self,
        latitude: f64,
        longitude: f64,
        days_back: i32,
    ) -> Result<NDVITrend> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(days_back as i64);
        
        let mut values = Vec::new();
        for date in date_range(start_date, end_date, Duration::days(5)) {
            values.push(self.get_ndvi(latitude, longitude, date)?);
        }
        
        // Fit linear regression
        let slope = calculate_trend_slope(&values);
        
        Ok(NDVITrend {
            current: values.last().unwrap().clone(),
            trend_direction: if slope > 0.001 { "IMPROVING" } else { "DEGRADING" },
            anomaly: values.last().unwrap() - values.first().unwrap(),
        })
    }

    /// Temperature adjustment based on vegetation
    pub fn vegetation_temperature_effect(
        &self,
        ndvi: f32,
    ) -> f32 {
        // Dense vegetation (NDVI > 0.5): -1.5 to -2.0°C cooling
        // Moderate (0.3-0.5): -0.5 to -1.0°C
        // Sparse (0.2-0.3): -0.2°C
        // Barren (< 0.2): 0°C
        
        match ndvi {
            n if n > 0.5 => -1.75,
            n if n > 0.3 => -0.75,
            n if n > 0.2 => -0.2,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NDVITrend {
    pub current: f32,
    pub trend_direction: &'static str,
    pub anomaly: f32,
}
```

### Data Download & Processing

```python
# Download Sentinel-2 NDVI from Copernicus
import requests
from sentinelhub import SentinelHub

class SentinelDownloader:
    def download_ndvi_monthly(self, bbox, year, month, output_dir):
        """Download monthly NDVI composite for region"""
        
        api = SentinelHub()
        
        # Request NDVI calculation
        response = api.get_data(
            bbox=bbox,
            time_range=(f'{year}-{month:02d}-01', f'{year}-{month:02d}-28'),
            product='NDVI',  # Sentinel-2 L2A
            resolution=10  # 10m
        )
        
        # Save as GeoTIFF
        response.save_tiff(f'{output_dir}/ndvi_{year}_{month:02d}.tif')
        
        # Create COG for efficient access
        subprocess.run([
            'gdalwarp',
            '-of', 'COG',
            '-co', 'COMPRESS=ZSTD',
            f'{output_dir}/ndvi_{year}_{month:02d}.tif',
            f'{output_dir}/ndvi_{year}_{month:02d}_COG.tif'
        ])
```

### Effort: 3-4 weeks
- Week 1: Sentinel data download pipeline, COG optimization
- Week 2: NDVI processing, trend calculation
- Week 3: PyO3 bindings, vegetation effects modeling
- Week 4: Testing, examples (drought/wildfire/agricultural use cases)

---

## Phase 4: Soil Data Integration - v0.5.5

### Requirements

**Data Sources**:
- SoilGrids (ISRIC) - 250m resolution, free
- HWSD (FAO) - 1km resolution, free

**Data Includes**: Texture, pH, organic matter, water holding capacity

### Implementation

```rust
// src/geospatial/soil.rs

#[derive(Debug, Clone)]
pub struct SoilProperties {
    pub texture: String,  // sand, silt, clay, loam, etc.
    pub sand_percent: f32,
    pub clay_percent: f32,
    pub silt_percent: f32,
    pub ph: f32,
    pub organic_matter_percent: f32,
    pub water_holding_capacity_mm: f32,  // mm/meter
    pub drainage_class: String,  // well, moderate, poor
    pub depth_cm: f32,
}

pub struct SoilService {
    soil_tiles: Arc<HashMap<String, Dataset>>,
}

impl SoilService {
    /// Get soil properties at coordinate
    pub fn get_soil_properties(
        &self,
        latitude: f64,
        longitude: f64,
        depth_cm: f32, // 0-30cm (topsoil), 30-100cm (subsoil)
    ) -> Result<SoilProperties> {
        let tile_id = self.get_tile_id(latitude, longitude);
        let dataset = self.load_soil_tile(&tile_id)?;
        
        // Read soil texture fractions
        let sand = dataset.read_band(1, latitude, longitude)?;
        let clay = dataset.read_band(2, latitude, longitude)?;
        let silt = 100.0 - sand - clay;
        
        // Classify texture
        let texture = self.classify_texture(sand, clay);
        
        // Read other properties
        let ph = dataset.read_band(3, latitude, longitude)?;
        let om = dataset.read_band(4, latitude, longitude)?;
        
        // Calculate water holding capacity
        // Clay-rich soils hold more water
        let whc = self.calculate_whc(sand, clay, om);
        
        Ok(SoilProperties {
            texture,
            sand_percent: sand,
            clay_percent: clay,
            silt_percent: silt,
            ph,
            organic_matter_percent: om,
            water_holding_capacity_mm: whc,
            drainage_class: self.estimate_drainage(texture),
            depth_cm,
        })
    }

    /// Estimate soil moisture
    pub fn estimate_soil_moisture(
        &self,
        soil: &SoilProperties,
        recent_rainfall_mm: f32,
        days_since_rain: i32,
        reference_et_mm: f32,
    ) -> f32 {
        // Simple water balance: rainfall - evapotranspiration
        // Adjusted for soil water holding capacity
        
        let max_available_water = soil.water_holding_capacity_mm;
        let daily_loss = reference_et_mm;
        let loss_since_rain = daily_loss * days_since_rain as f32;
        
        let soil_moisture = (recent_rainfall_mm - loss_since_rain)
            .max(0.0)
            .min(max_available_water);
        
        // Return as percentage
        (soil_moisture / max_available_water) * 100.0
    }

    fn calculate_whc(&self, sand: f32, clay: f32, om: f32) -> f32 {
        // Water holding capacity (mm/meter)
        // Sandy: 50-100mm
        // Loam: 150-200mm  
        // Clay: 200-300mm
        // Organic matter adds 20-30mm per 1% OM
        
        let base_whc = if sand > 60.0 {
            75.0
        } else if clay > 40.0 {
            250.0
        } else {
            175.0
        };
        
        let om_contribution = om * 25.0;
        
        base_whc + om_contribution
    }

    fn classify_texture(&self, sand: f32, clay: f32) -> String {
        // USDA soil texture triangle
        match (sand, clay) {
            (s, _) if s > 90.0 => "Sand",
            (s, c) if s > 70.0 && c < 10.0 => "Loamy Sand",
            (s, c) if c < 27.0 && s > 52.0 => "Sandy Loam",
            (s, c) if c >= 27.0 && c < 40.0 && s > 20.0 => "Clay Loam",
            (_, c) if c > 40.0 => "Clay",
            (s, c) if s >= 50.0 && c >= 27.0 && c < 40.0 => "Sandy Clay Loam",
            _ => "Loam",
        }.to_string()
    }
}
```

### Effort: 2-3 weeks
- Week 1: SoilGrids data import, texture classification
- Week 2: Water holding capacity, moisture calculation
- Week 3: PyO3 bindings, irrigation recommendations

---

## Phase 5: Disaster Risk Layers - v0.6.0

### Flood Risk

```rust
// src/geospatial/hazards/flood_risk.rs

pub struct FloodRiskService {
    dem: ElevationService,
    rainfall: RainfallService,
    landuse: LandUseService,
}

impl FloodRiskService {
    pub fn calculate_flood_risk(
        &self,
        latitude: f64,
        longitude: f64,
        rainfall_24h_mm: f32,
    ) -> FloodRisk {
        let elevation = self.dem.get_elevation(latitude, longitude)?;
        let slope = self.dem.calculate_slope(latitude, longitude)?;
        let landuse = self.landuse.get_landuse(latitude, longitude)?;
        
        // Flood risk factors
        let elevation_factor = if elevation < 50.0 { 1.0 } else { 0.5 }; // Low elevation = risk
        let slope_factor = if slope < 2.0 { 1.0 } else { 0.3 }; // Flat = water accumulates
        let drainage_factor = match landuse {
            "urban" => 1.5,  // Impervious surfaces
            "agricultural" => 0.8,  // Some infiltration
            "forest" => 0.3,  // Good drainage
            _ => 0.5,
        };
        
        let rainfall_factor = (rainfall_24h_mm / 100.0).min(1.0);
        
        let flood_risk_score = (elevation_factor + slope_factor + drainage_factor + rainfall_factor) / 4.0 * 100.0;
        
        FloodRisk {
            score: flood_risk_score,
            category: self.categorize_risk(flood_risk_score),
            factors: vec![
                ("elevation", elevation_factor),
                ("slope", slope_factor),
                ("drainage", drainage_factor),
                ("rainfall", rainfall_factor),
            ],
        }
    }
}
```

### Effort: 2-3 weeks per hazard type

---

## Deployment Architecture

### Docker Compose Setup

```yaml
# docker-compose.yml
version: '3.8'

services:
  # Geospatial data tile server
  tile-server:
    image: maptiler/tileserver-gl:latest
    volumes:
      - /data/tiles:/data  # Pre-processed GeoTIFF tiles
    ports: [8080:8080]

  # PostGIS for vector queries
  postgis:
    image: postgis/postgis:latest
    environment:
      POSTGRES_PASSWORD: password
    volumes:
      - /data/osm:/data  # OSM data
    ports: [5432:5432]

  # Redis cache
  redis:
    image: redis:7-alpine
    ports: [6379:6379]

  # PyWeatherEnriched with geospatial
  weather-api:
    build: .
    environment:
      TILE_SERVER_URL: http://tile-server:8080
      POSTGIS_URL: postgres://postgis:5432
      REDIS_URL: redis://redis:6379
    ports: [8765:8765]
    depends_on: [tile-server, postgis, redis]
```

### Storage Layout

```
/data/
├── srtm/
│   ├── -60_-180_COG.tif      (1°×1° tiles)
│   ├── -60_-179_COG.tif
│   └── ... (143,920 tiles total)
├── osm/
│   ├── buildings.geojson
│   ├── landuse.geojson
│   └── buildings.index       (R-tree index)
├── sentinel/
│   ├── ndvi_2024_01_COG.tif   (monthly composites)
│   ├── ndvi_2024_02_COG.tif
│   └── ... (12 per year)
├── soil/
│   ├── soilgrids_250m_COG.tif
│   └── hwsd_1km_COG.tif
└── indices/
    ├── tile38.db             (Geospatial index)
    └── rtree.db              (R-tree for fast spatial queries)

Total: ~600 GB (with compression)
```

---

## Performance Benchmarks

### Expected Query Latency

| Operation | Latency | Notes |
|-----------|---------|-------|
| Get elevation | 1-5ms | From Redis/GeoTIFF |
| Get OSM buildings (1km radius) | 10-50ms | Spatial index query |
| Get NDVI | 5-10ms | From cached tile |
| Get soil properties | 2-5ms | Interpolation |
| Calculate UHI | 20-100ms | Aggregation over buildings |
| Flood risk calculation | 50-200ms | Multiple factors |

### Throughput

- Single enrichment (weather + geospatial): <500ms
- Batch enrichment (1000 rows): <100s (parallel)
- Real-time stream (100 msgs/sec): <10ms per record

---

## Cost Estimates

### Development
- Phase A (Elevation + OSM + NDVI): 10 weeks, 2 engineers = $100K
- Phase B (Soil + Hazards): 8 weeks, 2 engineers = $80K
- **Total**: ~$200K engineering

### Infrastructure  
- Data storage (600 GB): $20-30/month
- Compute (processing, serving): $50-100/month
- **Annual**: ~$1K infrastructure cost

### Data
- All sources are free/open (SRTM, OSM, Sentinel, SoilGrids)
- Update costs: Sentinel updates ~$100/month for storage

---

## Success Metrics

- **Accuracy**: 20-30% improvement in temperature/wind predictions
- **Latency**: <500ms for single enrichment, <100s for 1K rows
- **Scalability**: Handle 100K+ enrichments/day
- **Market**: 3-5x expansion to agriculture, disaster response

---

## Recommendation

**Start with Phase A (Elevation + OSM + NDVI)** for v0.5.0:
- Highest impact on accuracy
- Enables core use cases (agriculture, urban planning)
- Reasonable engineering effort (10 weeks)
- Foundation for subsequent phases

**Then Phase B-C** for domain specialization (agriculture, disaster response)
