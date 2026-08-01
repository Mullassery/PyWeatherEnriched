# PyWeatherEnriched Onboarding - Quick Reference

## At a Glance

```
PyWeatherEnriched offers 5 ways to configure your enrichment pipeline
```

### 1️⃣ CLI Wizard (Interactive)
```bash
$ pyweatherenriched-setup
```
- 📋 Answer questions about your needs
- 🎯 Get use case suggestions
- ✅ Auto-generate config.toml
- 📝 Create sample code

### 2️⃣ Python Builder (Programmatic)
```python
from pyweatherenriched import EnrichmentBuilder

enricher = (EnrichmentBuilder()
    .with_weather(cache_size=5000)
    .with_reverse_geocoding(detail_level="Standard")
    .with_spatial(elevation=True, uhi=True)
    .with_data_sources(source_type="local_file", path="/data")
    .build())
```
- 🐍 Pure Python fluent API
- ✅ Great for testing & CI/CD
- 🔧 Programmatic control
- 💾 `enricher.save("config.toml")`

### 3️⃣ Config Validator (Verification)
```bash
$ pyweatherenriched-validate check
$ pyweatherenriched-validate detect
$ pyweatherenriched-validate show
```
- ✅ Validate TOML syntax
- 🔍 Auto-detect data sources
- 🐛 Debug configuration issues
- 📊 Check data availability

### 4️⃣ Setup Script (Batch/CI)
```bash
$ cat config.json | pyweatherenriched-setup
```
- 🤖 Non-interactive mode
- 🔗 JSON input format
- 🚀 CI/CD friendly
- 📦 Infrastructure as code

### 5️⃣ CLI Dashboard (Monitoring)
```bash
$ pyweatherenriched-dashboard
```
- 📊 View current configuration
- 🎯 Monitor performance
- 🔄 Live cache statistics
- ⌨️ Tab navigation with ◀/▶

---

## Quick Start Decision Tree

```
Q: Are you configuring for the first time?
├─ YES → Run CLI Wizard: pyweatherenriched-setup
└─ NO
   Q: Are you a developer?
   ├─ YES → Use Python Builder: EnrichmentBuilder()
   └─ NO
      Q: Do you need to verify configuration?
      ├─ YES → Run Validator: pyweatherenriched-validate check
      └─ NO
         Q: Are you setting up in CI/CD?
         ├─ YES → Use Setup Script: cat config.json | pyweatherenriched-setup
         └─ NO → Monitor with Dashboard: pyweatherenriched-dashboard
```

---

## Use Case Templates

### 🌾 Agriculture
```bash
# Wizard selection
Delivery/Logistics
✅ Weather ✅ Elevation ✅ Vegetation ✅ Soil ✅ Flood Risk

# Python
EnrichmentBuilder().with_use_case("agriculture")
  .with_spatial(elevation=T, uhi=F, vegetation=T, soil=T)
```

### 🚚 Delivery/Logistics
```bash
# Wizard selection
Delivery/Logistics
✅ Weather ✅ Reverse Geocoding ✅ Elevation ✅ UHI

# Python
EnrichmentBuilder().with_use_case("delivery")
  .with_reverse_geocoding(detail_level="Extended")
  .with_spatial(elevation=F, uhi=T)
```

### 🏙️ Urban Planning
```bash
# Wizard selection
Urban Planning
✅ Elevation ✅ UHI ✅ Vegetation ❌ Soil ❌ Flood Risk

# Python
EnrichmentBuilder().with_use_case("urban_planning")
  .with_spatial(elevation=T, uhi=T, vegetation=T)
```

### ⚡ Energy/Utilities
```bash
# Wizard selection
Energy/Utilities
✅ Elevation ✅ Weather ✅ Flood Risk

# Python
EnrichmentBuilder().with_use_case("energy")
  .with_spatial(elevation=T, uhi=F, flood_risk=T)
```

### 🏥 Healthcare
```bash
# Wizard selection
Healthcare
✅ Weather ✅ Reverse Geocoding

# Python
EnrichmentBuilder().with_use_case("healthcare")
  .with_reverse_geocoding(enabled=T)
  .with_weather(enabled=T)
```

---

## Configuration Options Quick Map

### Weather Configuration
| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `enabled` | true/false | true | Enable weather enrichment |
| `cache_size` | 1K-100K | 5000 | Cache entries |
| `cache_ttl_days` | 1-365 | 30 | Cache time to live |
| `api_key` | string | optional | Weather API key |

### Reverse Geocoding
| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `enabled` | true/false | true | Enable geocoding |
| `detail_level` | Minimal/Standard/Extended/Complete | Standard | Output format |
| `primary_source` | osm/google/usps | osm | Primary data source |
| `fallback_sources` | [osm,google,usps] | [google,usps] | Fallback sources |

### Spatial Details
| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `elevation` | true/false | true | Enable elevation (CRITICAL) |
| `uhi` | true/false | true | Enable UHI (CRITICAL) |
| `vegetation` | true/false | false | Enable vegetation |
| `soil` | true/false | false | Enable soil data |
| `flood_risk` | true/false | false | Enable flood risk |

### Data Sources
| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `source_type` | local_file/redis/s3/http/hybrid | local_file | Data source type |
| `path` | string | /data/geospatial | Local file path |
| `redis_url` | string | redis://localhost:6379 | Redis connection |
| `s3_bucket` | string | optional | AWS S3 bucket |
| `hybrid_sources` | [type1,type2,...] | optional | Fallback sources |

---

## Common Commands Cheat Sheet

```bash
# Setup & Configuration
pyweatherenriched-setup              # Interactive wizard
pyweatherenriched-setup < setup.json # Batch mode

# Validation
pyweatherenriched-validate check                        # Validate config
pyweatherenriched-validate detect                       # Auto-detect sources
pyweatherenriched-validate show                         # Show config
pyweatherenriched-validate --config custom.toml check   # Validate custom file

# Monitoring
pyweatherenriched-dashboard          # Launch TUI dashboard

# Python
python -c "from pyweatherenriched import EnrichmentBuilder; ..."
```

---

## Configuration File Location Priority

When not specified, config is looked up in this order:
1. `./enrichment.toml` (current directory)
2. `~/.config/pyweatherenriched/config.toml` (user home)
3. `/etc/pyweatherenriched/config.toml` (system)
4. Built-in defaults

---

## Troubleshooting

### ❌ "Config file not found"
```bash
# Solution
$ pyweatherenriched-setup
$ pyweatherenriched-validate show
```

### ❌ "Data sources not available"
```bash
# Solution
$ pyweatherenriched-validate detect
# Follow recommendations to configure cloud/local data
```

### ❌ "Python import error"
```bash
# Solution
pip install --upgrade pyweatherenriched
```

### ❌ "Reverse geocoding returns low confidence"
```bash
# Solution
$ pyweatherenriched-validate detect
# Configure Google Maps API for fallback
```

---

## Next Steps

1. **New to PyWeatherEnriched?** → [ONBOARDING_GUIDE.md](ONBOARDING_GUIDE.md)
2. **Need detailed config?** → [GEOSPATIAL_IMPLEMENTATION_GUIDE.md](GEOSPATIAL_IMPLEMENTATION_GUIDE.md)
3. **Reverse geocoding details?** → [REVERSE_GEOCODING_GUIDE.md](REVERSE_GEOCODING_GUIDE.md)
4. **Integration research?** → [GEOSPATIAL_INTEGRATION_RESEARCH.md](GEOSPATIAL_INTEGRATION_RESEARCH.md)
5. **Technology roadmap?** → [GEOSPATIAL_TECH_ROADMAP.md](GEOSPATIAL_TECH_ROADMAP.md)

---

## Support

- 📖 Documentation: [ONBOARDING_GUIDE.md](ONBOARDING_GUIDE.md)
- 🐛 Issues: GitHub issues
- 💬 Questions: Open a discussion
- 📧 Email: mullassery@gmail.com
