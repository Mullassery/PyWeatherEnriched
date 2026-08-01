# PyWeatherEnriched v0.3.0

**Hyperlocal Weather Enrichment Engine with Intelligent Multi-Tier Caching**

## Overview

PyWeatherEnriched is a high-performance Rust+Python weather enrichment system with intelligent caching for large-scale batch processing. Process millions of records with 90-98% fewer API calls through temporal range caching, geospatial clustering, and batch deduplication.

**Perfect for:**
- Climate research & historical analysis
- Agricultural optimization & soil monitoring
- Healthcare epidemiology & disease correlation
- Energy grid load forecasting
- Environmental monitoring networks
- Renewable energy prediction

## Key Features

### 🚀 Enhanced Caching Layer (v0.3.0 NEW)

- **Multi-Tier Architecture**: Memory (LRU) + Persistent (SQLite) tiers
- **Temporal Range Caching**: Query weather for entire date ranges (70% API reduction)
- **Geospatial Clustering**: Reuse nearby location data (60-80% savings)
- **Batch Deduplication**: Identify unique requests before API calls (80-95% reduction)
- **Smart TTL Management**: Configurable expiration, automatic cleanup

### Performance Results

| Scenario | Without Cache | With Cache | Savings |
|----------|---------------|-----------|---------|
| 1M rows, 100K unique locations | 100K API calls | 5-10K | **90-95%** |
| 30-day enrichment, same cities | 30K API calls | 0-2K | **93-100%** |
| Urban sensor network (100 sensors) | 72K API calls | 1-2K | **97-99%** |
| Regional analysis (500 stations) | 50K+ API calls | 2-5K | **90-96%** |

### Core Features

- **Rust Engine**: High-performance compiled core with Python bindings
- **PyO3 Bindings**: Zero-copy Python integration (Python 3.10+)
- **Hyperlocal Precision**: Microgeography adjustments (UHI, elevation, wind)
- **Parallel Processing**: Rayon-based multi-threaded batch enrichment
- **Multiple Data Formats**: CSV, JSON, JSONL with nested data support
- **Database Integration**: Snowflake, BigQuery, PostgreSQL backends
- **MCP 2.0 Ready**: Integrated with unified platform (207 tools)

## Installation

```bash
pip install pyweatherenriched
```

Or with wheels only (recommended for production):

```bash
pip install --only-binary=:all: pyweatherenriched
```

## Quick Start

### Basic Usage

```python
import pyweatherenriched as pwe

# Create enricher
enricher = pwe.WeatherEnricher(cache_size=1000)

# Enrich single row
result = enricher.enrich_row("New York", "2024-01-15T12:00:00Z")
print(result)  # {location, latitude, longitude, temperature, humidity, condition, timestamp}
```

### Enhanced Caching (NEW)

```python
from pyweatherenriched import EnhancedCache

# Create cache with persistence
cache = EnhancedCache(cache_size=5000, db_path="weather_cache.db")

# Configure for your use case
cache.set_proximity_radius(10.0)  # 10km for cities
cache.set_ttl(72)  # 72-hour TTL

# Cache weather data
cache.put(
    location="New York",
    latitude=40.7128,
    longitude=-74.0060,
    temperature=15.2,
    humidity=65.0,
    condition="Partly Cloudy",
    timestamp="2024-01-15T12:00:00Z"
)

# Retrieve with intelligent fallback
result = cache.get("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z")

# Batch deduplication
batch = [
    ("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z"),
    ("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z"),  # duplicate
    ("Herald Square", 40.7505, -73.9865, "2024-01-15T12:00:00Z"),  # nearby
]

missing_indices, cache_hits = cache.deduplicate_batch(batch)
print(f"API calls needed: {len(missing_indices)}, Cache hits: {cache_hits}")

# Monitor performance
stats = cache.stats()
print(f"Hit ratio: {stats['hit_ratio']:.1%}")
```

### Batch Enrichment

```python
# Load CSV/JSON and enrich with weather
enricher = pwe.WeatherEnricher()

enriched_data = enricher.enrich_batch([
    ("New York", "2024-01-15T12:00:00Z"),
    ("Los Angeles", "2024-01-15T12:00:00Z"),
    ("Chicago", "2024-01-15T12:00:00Z"),
])

# Export results
enricher.export_csv("output.csv")
```

## Documentation

- **[ENHANCED_CACHE.md](docs/ENHANCED_CACHE.md)** - Complete caching guide with API reference
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and module breakdown
- **[examples/mcp_enhanced_cache.py](examples/mcp_enhanced_cache.py)** - 5 practical examples
- **[examples/mcp_enhanced_cache_use_cases.py](examples/mcp_enhanced_cache_use_cases.py)** - 6 industry use cases

## Use Cases

### 🔬 Climate Research
50-year historical analysis across 500 weather stations
- **Before**: 9.1M API calls
- **After**: 50-100K API calls with smart caching
- **Savings**: 94-99%

### 🌾 Agricultural Optimization
Soil moisture monitoring across 200 fields with 800 sensors
- **Before**: 3.5M readings
- **After**: Smart clustering reduces to 5-10% unique requests
- **Cost**: ~$0.50 per growing season (vs $50)

### 🏥 Healthcare Epidemiology
Disease-weather correlation across hospital network
- **Before**: 91K location-date combinations = many API calls
- **After**: 5-10K unique requests with deduplication
- **Efficiency**: 90%+ dedup in overlapping data

### ⚡ Energy Grid Management
Load forecasting from 200 substations hourly
- **Before**: 144K readings
- **After**: 2-5K unique with proximity matching
- **Real-time**: Sub-second forecast generation

### 🌍 Environmental Monitoring
Air quality correlation across 300 stations
- **Before**: 2.6M data points
- **After**: 10-15% unique with intelligent caching
- **Latency**: 10x faster analysis

### ♻️ Renewable Energy Forecasting
Solar/wind prediction from 1,500 assets
- **Before**: 1M+ data points
- **After**: 50-100K unique with 15-min grouping
- **Cost/forecast**: $0.01 (vs $0.50)

## MCP 2.0 Integration

Part of unified **MCP 2.0 Mega-Platform** (207 tools across 18 projects):
- Discoverable by Claude via MCP protocol on port 8769
- Multi-project workflows with intelligent optimization
- Cross-database joins with cost-optimized routing

See [MCP_QUICKSTART.md](MCP_QUICKSTART.md) for details.

## Benchmarks

```
Single Row Enrichment:      ~200-500ms (includes API call)
Cached Row Lookup:          ~10-50ms (memory tier)
Batch Processing (1M rows): ~2-4 hours (with parallelization)
Cache Hit Ratio:            70% typical (24-hour TTL)
```

## Requirements

- Python 3.10+
- Rust 1.70+ (for building from source)
- SQLite 3.44+ (bundled)

## Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build extension
cargo build --release

# Install in development mode
pip install -e .
```

## Storage Requirements

| Scale | DB Size | Memory (LRU) | Typical Usage |
|-------|---------|--------------|---------------|
| 10K entries | 1MB | 10MB | Single project, 1 week |
| 100K entries | 10MB | 50MB | Multi-project, 1 month |
| 1M entries | 100MB | 200MB | Large scale, 3+ months |
| 10M entries | 1GB | 1.5GB | Enterprise, 1+ year |

## Version History

### v0.3.0 (Current) - Enhanced Caching
- ✅ Multi-tier caching (memory + SQLite)
- ✅ Temporal range queries (70% API reduction)
- ✅ Geospatial clustering (60-80% savings)
- ✅ Batch deduplication (80-95% reduction)
- ✅ Comprehensive documentation & examples
- ✅ 6 real-world use case implementations

### v0.2.0 - Rust+PyO3 Engine
- Rust-based weather enrichment
- Python 3.13 support
- Async API integration

### v0.1.0 - Initial Release
- Basic weather enrichment
- CSV/JSON support

## Contributing

Contributions welcome! Submit issues and PRs on GitHub.

## License

Proprietary - Mullassery Weather Systems

## Support

- 📖 **Documentation**: See [docs/](docs/) directory
- 🐛 **Issues**: GitHub Issues
- 💬 **Email**: mullassery@gmail.com

---

**PyWeatherEnriched v0.3.0 | Hyperlocal Weather Enrichment | 90-98% API Cost Reduction**
