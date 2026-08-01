# Geospatial Integration Guide

Full implementation of critical geospatial layers (Elevation + UHI) with framework for optional layers.

---

## Architecture Overview

```
PyWeatherEnriched (Core)
    ↓
WeatherEnricher (existing)
    ↓
GeospatialEnricher (NEW)
    ├─ CRITICAL (always loaded)
    │   ├─ ElevationService (SRTM)
    │   └─ UHIService (OSM)
    └─ OPTIONAL (load on demand)
        ├─ VegetationService (framework stub)
        ├─ SoilService (framework stub)
        └─ FloodRiskService (framework stub)
```

---

## File Structure

```
src/geospatial/
├── mod.rs                    # Main module, exports GeospatialEnricher
├── config.rs                 # Configuration, data source options
├── data_source.rs            # Abstract loader (local, Redis, S3, HTTP, hybrid)
├── elevation.rs              # CRITICAL: SRTM elevation implementation
├── urban_heat_island.rs      # CRITICAL: OSM-based UHI modeling
└── optional.rs               # Framework stubs for optional layers
```

---

## Configuration Options

Users can configure where geospatial data comes from:

### Option 1: Local Files (Default)
```toml
# geospatial.toml
[elevation]
source = "local_file"
base_path = "/data/geospatial"
file_pattern = "srtm/{lat}_{lon}_COG.tif"

[urban_heat_island]
source = "local_file"
base_path = "/data/geospatial"
file_pattern = "osm/{lat}_{lon}.geojson"

[vegetation]
enabled = false  # Optional - not loaded by default
```

### Option 2: Redis Cache (Shared)
```toml
[elevation]
source = "redis"
url = "redis://localhost:6379"
key_prefix = "geo:elevation:"

[urban_heat_island]
source = "redis"
url = "redis://localhost:6379"
key_prefix = "geo:uhi:"
```

### Option 3: Cloud Storage (S3/GCS)
```toml
[elevation]
source = "s3"
bucket = "my-geospatial"
region = "us-west-2"
prefix = "srtm/"
file_pattern = "{lat}_{lon}_COG.tif"
```

### Option 4: Hybrid (Try multiple sources)
```toml
[elevation]
source = "hybrid"
sources = [
    { type = "local_file", base_path = "/data", pattern = "srtm/{lat}_{lon}.tif" },
    { type = "http", url = "https://api.example.com/", pattern = "{lat}_{lon}.tif" }
]
```

---

## Python API

### Basic Usage (Critical Layers Only)

```python
from pyweatherenriched import GeoSpatialEnricher

# Create enricher with default local file configuration
enricher = GeoSpatialEnricher(
    config_path="geospatial.toml"  # or None for defaults
)

# Enrich location with elevation and UHI
result = enricher.enrich(
    latitude=40.7128,
    longitude=-74.0060
)

# Result includes critical layers
print(result)
# {
#   'elevation': {
#     'elevation_m': 10.0,
#     'lapse_rate_adjustment_c': -0.065,
#     'terrain_roughness': 0.1
#   },
#   'urban_heat_island': {
#     'building_density_percent': 85.0,
#     'average_building_height_m': 25.0,
#     'uhi_effect_c': 2.3,
#     'location_type': 'dense_urban'
#   },
#   'vegetation': None,  # Not requested
#   'soil': None,
#   'flood_risk': None
# }
```

### With Optional Layers

```python
# Request specific optional layers
result = enricher.enrich(
    latitude=40.7128,
    longitude=-74.0060,
    optional_layers=['vegetation', 'soil']  # Only these are loaded
)

# Result now includes optional data if configured
print(result['vegetation'])
# {
#   'ndvi': 0.35,
#   'vegetation_type': 'urban_park',
#   'cooling_effect_c': -0.5
# }
```

### Integration with WeatherEnricher

```python
from pyweatherenriched import WeatherEnricher

# Create weather enricher (existing)
weather_enricher = WeatherEnricher(cache=distributed_cache)

# Create geospatial enricher (new)
geo_enricher = GeoSpatialEnricher(config_path="geospatial.toml")

# Combined enrichment
weather = weather_enricher.enrich("New York", "2024-01-15T12:00:00Z")
geospatial = geo_enricher.enrich(40.7128, -74.0060)

# Merge results
enriched = {
    **weather,
    **geospatial,
    'temperature_adjusted': (
        weather['temperature'] 
        + geospatial['elevation']['lapse_rate_adjustment_c']
        + geospatial['urban_heat_island']['uhi_effect_c']
    )
}

print(enriched)
# {
#   'temperature': 15.0,
#   'elevation_m': 10.0,
#   'uhi_effect_c': 2.3,
#   'temperature_adjusted': 17.3,
#   ...
# }
```

### Batch Processing

```python
# Enrich multiple locations
locations = [
    (40.7128, -74.0060, "New York"),
    (34.0522, -118.2437, "Los Angeles"),
    (41.8781, -87.6298, "Chicago"),
]

results = []
for lat, lon, name in locations:
    geo = geo_enricher.enrich(lat, lon, optional_layers=['elevation', 'uhi'])
    results.append({
        'name': name,
        'lat': lat,
        'lon': lon,
        **geo
    })

# Filter by UHI effect
high_uhi = [r for r in results if r['urban_heat_island']['uhi_effect_c'] > 2.0]
print(f"Locations with strong UHI effect: {[r['name'] for r in high_uhi]}")
```

### Performance Monitoring

```python
# Check cache statistics
stats = geo_enricher.cache_stats()
print(stats)
# {
#   'elevation_hits': 1500,
#   'elevation_misses': 50,
#   'elevation_hit_ratio': 0.968,
#   'uhi_hits': 1200,
#   'uhi_misses': 100,
#   'uhi_hit_ratio': 0.923,
#   'total_queries': 1550
# }

# Clear cache
geo_enricher.clear_cache()
```

---

## Rust Integration (For Extension)

### Adding a New Optional Layer

To implement a new optional layer (e.g., Air Quality):

```rust
// 1. Add to geospatial/mod.rs
pub struct AirQualityData {
    pub pm25: f32,
    pub pm10: f32,
    pub aqi: i32,
}

// 2. Create air_quality.rs
pub struct AirQualityService { ... }
impl AirQualityService {
    pub fn get_data(&self, lat: f64, lon: f64) -> Result<AirQualityData> { ... }
}

// 3. Add to OptionalServices in optional.rs
pub struct OptionalServices {
    pub vegetation: VegetationService,
    pub soil: SoilService,
    pub flood_risk: FloodRiskService,
    pub air_quality: AirQualityService,  // NEW
}

// 4. Add to GeospatialContext
pub struct GeospatialContext {
    pub elevation: ElevationData,
    pub urban_heat_island: UHIData,
    pub vegetation: Option<VegetationData>,
    pub soil: Option<SoilData>,
    pub flood_risk: Option<FloodRiskData>,
    pub air_quality: Option<AirQualityData>,  // NEW
}

// 5. Handle in enrich() method
if requested_layers.contains(&"air_quality") {
    air_quality = Some(self.optional_services.air_quality.get_data(lat, lon)?);
}
```

---

## Data Preparation

### Preparing Elevation Data (SRTM)

```bash
# Download SRTM tiles from USGS
# Expected format: GeoTIFF files in 1°×1° tiles
# Example: /data/srtm/40_-74_COG.tif (latitude_longitude)

# Convert to Cloud Optimized GeoTIFF (COG) for faster access
for f in *.tif; do
    gdalwarp -of COG -co COMPRESS=ZSTD "$f" "${f%.tif}_COG.tif"
done
```

### Preparing OSM Data (Buildings)

```bash
# Download OSM extract for your region
# Convert to GeoJSON with buildings only
ogr2ogr -f GeoJSON \
    -where "building IS NOT NULL" \
    buildings.geojson \
    building.shp

# Compress for storage
gzip buildings.geojson
```

---

## Deployment Example (Docker)

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features geospatial

FROM ubuntu:22.04
WORKDIR /app
COPY --from=builder /app/target/release/pyweatherenriched .

# Copy geospatial data
COPY data/srtm /data/srtm
COPY data/osm /data/osm

# Copy config
COPY geospatial.toml .

EXPOSE 8765
CMD ["./pyweatherenriched", "--geospatial-config", "geospatial.toml"]
```

---

## Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Get elevation | 1-5ms | From cache or local file |
| Get UHI data | 5-50ms | OSM parsing required |
| Full enrichment | <100ms | Both critical layers |
| With vegetation | +5-20ms | If requested |
| Batch (1000 rows) | <2s | Parallel processing |

---

## Future: Optional Layers Implementation

When implementing optional layers, follow this pattern:

**Vegetation (NDVI)**
- Data: Sentinel-2 monthly composites
- Processing: Raster value lookup + interpolation
- Effort: 2-3 weeks
- Value: Drought detection, cooling effects

**Soil Data**
- Data: SoilGrids or HWSD
- Processing: Raster value lookup + classification
- Effort: 1-2 weeks
- Value: Irrigation recommendations

**Flood Risk**
- Data: Derived from DEM + rainfall + land use
- Processing: Multi-factor modeling
- Effort: 2-3 weeks
- Value: Disaster early warning

---

## API Stability

**Critical layers** (Elevation + UHI):
- ✅ Stable, production-ready
- ✅ Backward compatible changes only

**Optional layers**:
- ⚠️ Framework stub only
- ⚠️ May change during implementation
- Users should not depend on these APIs yet

---

## Support & Contributing

To add a new geospatial layer:
1. Open an issue describing the layer and use case
2. Implement following the pattern in `optional.rs`
3. Submit PR with tests and documentation
4. Review and merge to main

For bugs or improvements, see CONTRIBUTING.md
