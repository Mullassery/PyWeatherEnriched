# PyWeatherEnriched Onboarding Guide

Complete guide to setting up PyWeatherEnriched with all four approaches: CLI wizard, Python builder, config validator, and setup script.

---

## Quick Start (CLI Wizard)

**Recommended for first-time users**

```bash
# Run interactive setup wizard
$ pyweatherenriched-setup

? 📍 Select your primary use case:
  > Agriculture (soil, elevation, vegetation, weather)
  > Delivery/Logistics (reverse geocoding, UHI, weather)
  > Urban Planning (UHI, vegetation, elevation)
  > Energy/Utilities (elevation, weather, flood risk)
  > Healthcare (weather, postal code lookup)
  > Custom (choose features manually)

? ⛅ Enable weather enrichment? (Y/n)
? 🗺️  Enable reverse geocoding (lat/lon → postal code)? (Y/n)
? 🌍 Which spatial layers do you need?
  [x] Elevation (terrain, lapse rate)
  [x] Urban Heat Island (building density)
  [ ] Vegetation/NDVI (cooling effects)
  [ ] Soil Properties (water capacity, pH)
  [ ] Flood Risk (hazard modeling)

? 💾 Where should geospatial data come from?
  > Local Files (filesystem)
  > Redis (distributed cache)
  > S3 (AWS cloud)
  > HTTP (API endpoint)
  > Hybrid (try multiple)

✅ Configuration Summary
📋 Use Case: Delivery/Logistics
⛅ Weather: ✅ Enabled (5,000 entry cache, 30 day TTL)
🗺️  Reverse Geocoding: ✅ Enabled (Standard detail, OpenStreetMap)
🌍 Spatial Details: ✅ Elevation + UHI
💾 Data Sources: Local Files (/data/geospatial)

? Save this configuration? (Y/n)

✅ Setup complete! Run 'pyweatherenriched-dashboard' to manage your configuration.
```

**Generated files:**
- `enrichment.toml` - Configuration file
- `sample_enrichment.py` - Sample code showing usage

---

## Approach 1: Interactive CLI Wizard

Perfect for: **First-time users, non-developers, configuration exploration**

### Run the wizard
```bash
$ pyweatherenriched-setup
```

### What it does
- 📋 Prompts for weather, geocoding, and spatial details
- 🎯 Suggests settings based on your use case
- 📁 Auto-detects available data sources
- ✅ Generates `enrichment.toml` config file
- 📝 Creates sample Python code

### Use case templates
The wizard provides pre-configured templates for:
- **Agriculture**: soil, elevation, vegetation, weather
- **Delivery/Logistics**: reverse geocoding, UHI, weather
- **Urban Planning**: UHI, vegetation, elevation
- **Energy/Utilities**: elevation, weather, flood risk
- **Healthcare**: weather, postal code lookup
- **Custom**: choose features manually

### Example output
```toml
# enrichment.toml (auto-generated)
[weather]
enabled = true
cache_size = 5000
cache_ttl_days = 30

[reverse_geocoding]
enabled = true
detail_level = "Standard"
primary_source = "osm"

[spatial]
elevation = true
urban_heat_island = true
vegetation = false
soil = false
flood_risk = false

[data_sources]
type = "local_file"
path = "/data/geospatial"
```

---

## Approach 2: Python Fluent Builder

Perfect for: **Developers, programmatic configuration, testing, CI/CD**

### Installation
```bash
pip install pyweatherenriched
```

### Basic usage
```python
from pyweatherenriched import EnrichmentBuilder

# Create enricher with fluent API
enricher = (EnrichmentBuilder()
    .with_weather(cache_size=10000, cache_ttl_days=30)
    .with_reverse_geocoding(
        enabled=True,
        detail_level="Standard",
        source="osm"
    )
    .with_spatial(
        elevation=True,
        uhi=True,
        vegetation=False
    )
    .with_data_sources(
        source_type="local_file",
        path="/data/geospatial"
    )
    .build())

# Get configuration as dict
config = enricher.build()
print(config)
# {
#   'weather': {'enabled': True, 'cache_size': 10000, ...},
#   'geocoding': {'enabled': True, ...},
#   'spatial': {'elevation': True, ...},
#   'data_sources': {'type': 'local_file', ...}
# }
```

### Save to TOML file
```python
enricher.save("my_config.toml")
```

### Use case examples

**Agriculture**
```python
enricher = (EnrichmentBuilder()
    .with_use_case("agriculture")
    .with_weather(cache_size=10000)
    .with_spatial(
        elevation=True,
        uhi=False,
        vegetation=True,  # NDVI for crop monitoring
        soil=True,        # Irrigation planning
        flood_risk=True   # Drainage assessment
    )
    .with_data_sources(source_type="s3", s3_bucket="agri-data")
    .build())
```

**Delivery/Logistics**
```python
enricher = (EnrichmentBuilder()
    .with_use_case("delivery")
    .with_reverse_geocoding(
        enabled=True,
        detail_level="Extended",  # Need county/neighborhood
        source="osm"
    )
    .with_spatial(
        elevation=False,  # Not critical
        uhi=True,         # Route optimization
    )
    .with_data_sources(
        source_type="hybrid",
        hybrid_sources=["local_file", "redis", "s3"]
    )
    .build())
```

**Urban Planning**
```python
enricher = (EnrichmentBuilder()
    .with_use_case("urban_planning")
    .with_spatial(
        elevation=True,   # Terrain analysis
        uhi=True,         # Heat map generation
        vegetation=True,  # Green space calculation
        soil=False,
        flood_risk=False
    )
    .with_data_sources(
        source_type="local_file",
        path="/data/cities"
    )
    .build())
```

### Advanced: Custom configuration
```python
builder = EnrichmentBuilder()

# Weather
builder.with_weather(
    api_key="your-api-key",
    cache_size=50000,
    cache_ttl_days=60,
    enabled=True
)

# Geocoding with fallbacks
builder.with_reverse_geocoding(
    enabled=True,
    detail_level="Complete",
    source="osm",
    fallback_sources=["google", "usps"]
)

# Selective spatial layers
builder.with_spatial(
    elevation=True,
    uhi=True,
    vegetation=True,
    soil=False,
    flood_risk=False
)

# Hybrid data sources
builder.with_data_sources(
    source_type="hybrid",
    path="/data/local",
    redis_url="redis://localhost:6379",
    s3_bucket="my-bucket"
)

enricher = builder.build()
enricher.save("production.toml")
```

---

## Approach 3: Config Validator

Perfect for: **Checking configurations, data source availability, debugging**

### Validate configuration file
```bash
$ pyweatherenriched-validate --config enrichment.toml check

🔍 Configuration Validation

✅ Config file found: enrichment.toml
✅ TOML syntax valid
✅ Weather enrichment configured
✅ Cache size: 5000 entries
✅ Reverse geocoding enabled
⚠️  OpenStreetMap data should be prepared (see docs)
✅ Elevation enabled (CRITICAL)
✅ Urban Heat Island enabled (CRITICAL)
✅ Vegetation disabled (OPTIONAL)
📂 Data source: Local Files
✅ Found: /data/geospatial/srtm
✅ Found: /data/geospatial/osm

Status: ✅ Valid
```

### Auto-detect data sources
```bash
$ pyweatherenriched-validate detect

🔍 Data Source Auto-Detection

🔍 Scanning for available data sources...

✅ Local files available at /data/geospatial
✅ Redis available at localhost:6379
❌ AWS credentials not configured
❌ GCP credentials not configured

Recommendations

✅ 2 data source(s) available
💡 Use first available source in auto-detection priority
```

### Show current configuration
```bash
$ pyweatherenriched-validate --config enrichment.toml show

enrichment.toml:

# PyWeatherEnriched Configuration
# Generated by setup wizard

[weather]
enabled = true
cache_size = 5000
cache_ttl_days = 30

[reverse_geocoding]
enabled = true
detail_level = "Standard"
primary_source = "osm"

[spatial]
elevation = true
urban_heat_island = true
vegetation = false
soil = false
flood_risk = false

[data_sources]
elevation_source = "local_file"
uhi_source = "local_file"
storage_path = "/data/geospatial"
```

---

## Approach 4: Setup Script (Batch/CI Mode)

Perfect for: **Automated deployment, CI/CD pipelines, infrastructure as code**

### Using setup script with JSON input
```bash
# Create configuration via JSON
$ cat > setup.json <<EOF
{
  "use_case": "delivery",
  "weather": {
    "enabled": true,
    "cache_size": 10000,
    "cache_ttl_days": 30
  },
  "reverse_geocoding": {
    "enabled": true,
    "detail_level": "Extended",
    "primary_source": "osm"
  },
  "spatial": {
    "elevation": true,
    "uhi": true,
    "vegetation": false,
    "soil": false,
    "flood_risk": false
  },
  "data_sources": {
    "type": "local_file",
    "path": "/data/geospatial"
  }
}
EOF

# Run non-interactive setup
$ pyweatherenriched-setup < setup.json

✅ Configuration saved to: enrichment.toml
📝 Sample code saved to: sample_enrichment.py
```

### CI/CD pipeline example
```yaml
# .github/workflows/deploy.yml
name: Deploy PyWeatherEnriched

on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Configure enrichment
        run: |
          pyweatherenriched-setup < config/setup.json
          pyweatherenriched-validate check
      
      - name: Deploy with configuration
        run: |
          docker build -t pyweatherenriched:latest .
          docker run -v /data:/data pyweatherenriched:latest
```

### Kubernetes manifest
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: pyweatherenriched-config
data:
  setup.json: |
    {
      "use_case": "delivery",
      "weather": {"enabled": true, "cache_size": 10000},
      "reverse_geocoding": {"enabled": true, "detail_level": "Standard"}
    }

---
apiVersion: v1
kind: Pod
metadata:
  name: pyweatherenriched
spec:
  containers:
  - name: enricher
    image: pyweatherenriched:latest
    volumeMounts:
    - name: config
      mountPath: /etc/pyweatherenriched
    - name: data
      mountPath: /data
  volumes:
  - name: config
    configMap:
      name: pyweatherenriched-config
  - name: data
    hostPath:
      path: /data/geospatial
```

---

## Approach 5: CLI Dashboard

Perfect for: **Monitoring configuration, viewing performance metrics, quick management**

### Launch dashboard
```bash
$ pyweatherenriched-dashboard

╔─ PyWeatherEnriched ─ Configuration Dashboard ────────────────────────────╗
│ Status: ✅ Active | Press 'h' for help, 'q' to quit                      │
├─────────────────────────────────────────────────────────────────────────┤
│ Overview | Weather | Geocoding | Spatial | Data Sources | Advanced      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│ Configuration Status                                                      │
│                                                                           │
│   ⛅  Weather Enrichment ........................... ✅ Enabled           │
│   🗺️  Reverse Geocoding ........................... ✅ Enabled           │
│   🌍 Spatial Details .............................. ✅ Elevation + UHI    │
│   💾 Data Sources ................................ ✅ Local Files        │
│                                                                           │
│ Quick Stats                                                               │
│                                                                           │
│   Use Case: Delivery/Logistics                                            │
│   Config File: enrichment.toml                                            │
│   Data Path: /data/geospatial                                             │
│                                                                           │
│ 💡 Tip: Use arrow keys to navigate tabs, 'q' to quit                     │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Dashboard tabs

**Overview**
- Quick configuration status
- Enabled features
- Use case summary

**Weather**
- Cache configuration
- Cache performance metrics
- Query statistics

**Geocoding**
- Reverse geocoding settings
- Data source status
- Cache hits and latency

**Spatial**
- Critical layers (always enabled)
- Optional layers (on-demand)
- Performance metrics

**Data Sources**
- Storage configuration
- Available data sources
- Fallback chains

**Advanced**
- System performance
- Cache efficiency
- Recent queries
- Uptime statistics

### Keyboard shortcuts
- `◀/▶` - Navigate between tabs
- `h` - Show help
- `q` - Quit dashboard

---

## Choosing Your Approach

| Approach | Best For | Learning Curve | Flexibility |
|----------|----------|-----------------|-------------|
| **CLI Wizard** | First-time users | Very easy | Medium |
| **Python Builder** | Developers, testing | Easy | Very high |
| **Config Validator** | Debugging, verification | Very easy | Low |
| **Setup Script** | CI/CD, automation | Easy | High |
| **CLI Dashboard** | Monitoring, management | Very easy | Low |

---

## Common Workflows

### Setup for production delivery platform
```bash
# 1. Run wizard for your use case
$ pyweatherenriched-setup
# Select: Delivery/Logistics
# Enable: Weather, Reverse Geocoding, Elevation, UHI
# Data Source: Hybrid (local + Redis + S3)

# 2. Validate configuration
$ pyweatherenriched-validate check

# 3. Monitor with dashboard
$ pyweatherenriched-dashboard
```

### Programmatic setup for testing
```python
# test_enrichment.py
from pyweatherenriched import EnrichmentBuilder

def test_agriculture_config():
    enricher = (EnrichmentBuilder()
        .with_use_case("agriculture")
        .with_weather(cache_size=1000)  # Small cache for tests
        .with_spatial(elevation=True, uhi=False, vegetation=True)
        .build())
    
    assert enricher['weather']['enabled']
    assert enricher['spatial']['vegetation']

def test_delivery_config():
    enricher = (EnrichmentBuilder()
        .with_use_case("delivery")
        .with_reverse_geocoding(detail_level="Extended")
        .build())
    
    assert enricher['geocoding']['enabled']
    assert enricher['geocoding']['detail_level'] == "Extended"
```

### CI/CD deployment
```bash
#!/bin/bash
# deploy.sh
set -e

# Generate config from JSON
pyweatherenriched-setup < config/production.json

# Validate
pyweatherenriched-validate check || exit 1

# Deploy
docker build -t enricher:latest .
docker push enricher:latest

echo "✅ Deployment complete"
```

---

## Recommended Reading

1. **First time?** → Start with [CLI Wizard](#approach-1-interactive-cli-wizard)
2. **Developer?** → Use [Python Builder](#approach-2-python-fluent-builder)
3. **Debugging issues?** → Run [Config Validator](#approach-3-config-validator)
4. **Setting up CI/CD?** → Use [Setup Script](#approach-4-setup-script-batchci-mode)
5. **Monitoring?** → Launch [CLI Dashboard](#approach-5-cli-dashboard)

For detailed configuration options, see [GEOSPATIAL_IMPLEMENTATION_GUIDE.md](GEOSPATIAL_IMPLEMENTATION_GUIDE.md).

For Python API details, see [REVERSE_GEOCODING_GUIDE.md](REVERSE_GEOCODING_GUIDE.md).
