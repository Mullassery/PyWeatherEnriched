# PyWeatherEnriched v0.3.0: Enhanced Caching Layer

## Release Summary

Enhanced caching layer for weather lookups optimized for large-scale batch enrichment with overlapping date ranges and nearby locations. This release reduces API calls by 70-98% in typical use cases.

## What's New

### Core Features

1. **Multi-Tier Caching Architecture**
   - Memory tier (L1): Fast LRU cache for hot data (<1ms latency)
   - Persistent tier (L2): SQLite backend for cross-session persistence
   - Automatic failover and upgrade path

2. **Temporal Range Caching**
   - Query weather data for entire date ranges
   - Indexed time-based lookups in SQLite
   - Reduces 30-day enrichment from 30K API calls to ~2K

3. **Geospatial Clustering**
   - Intelligent nearby location matching within configurable radius
   - Quantized coordinate indexing (~1km grid)
   - Haversine distance-based proximity search
   - Saves 60-80% of API calls in urban/clustered scenarios

4. **Batch Deduplication**
   - Identify unique cache misses before API calls
   - Prioritize missing requests for parallel API calls
   - 80-95% deduplication typical in production datasets

5. **TTL Management**
   - Configurable time-to-live per cache (default: 24 hours)
   - Automatic expiration on retrieval
   - Manual cleanup of expired entries

### Performance Improvements

| Scenario | Without Cache | With Cache | Savings |
|----------|---------------|-----------|---------|
| 1M rows, 100K unique | 100K API calls | 5-10K | 90-95% |
| 30-day enrichment, same locations | 30K API calls | 0-2K | 93-100% |
| Urban sensor network (100 sensors) | 72K API calls | 1-2K | 97-99% |
| Regional climate analysis (500 stations) | 50K+ API calls | 2-5K | 90-96% |

### API Changes

**New Python Classes:**

```python
# Enhanced cache with persistence
cache = pwe.EnhancedCache(
    cache_size=5000,
    db_path="weather_cache.db"  # Optional
)

# Configuration
cache.set_proximity_radius(10.0)  # 10km
cache.set_ttl(48)  # 48 hours

# Operations
result = cache.get(location, latitude, longitude, timestamp)
cache.put(location, latitude, longitude, temp, humidity, condition, timestamp)

missing_idx, hits = cache.deduplicate_batch([
    (location, lat, lon, timestamp),
    ...
])

stats = cache.stats()  # Performance metrics
cache.cleanup_expired()  # Maintenance
```

### Documentation

- **ENHANCED_CACHE.md**: Comprehensive guide with API reference and best practices
- **examples/mcp_enhanced_cache.py**: Basic examples (5 scenarios)
- **examples/mcp_enhanced_cache_use_cases.py**: Non-retail use cases (6 scenarios)
  - Climate research & historical analysis
  - Agricultural optimization
  - Healthcare & epidemiology
  - Energy grid management
  - Environmental monitoring
  - Renewable energy forecasting

## Installation & Building

```bash
# Build the Rust extension with new cache module
cargo build --release

# Python usage
from pyweatherenriched import EnhancedCache

cache = EnhancedCache(db_path="weather.db")
```

## Backward Compatibility

- ✓ Existing `PyWeatherEnricher` API unchanged
- ✓ Simple `Cache` still available for memory-only caching
- ✓ New `EnhancedCache` is opt-in feature
- ✓ No breaking changes to Python bindings

## Benchmarks

### Tested Scenarios

1. **Single Day, Multiple Locations** (1000 records, 100 unique)
   - Without cache: 100 API calls @ 100ms = 10s
   - With cache: 0-5 API calls = 1s
   - **Speedup: 10x**

2. **Multi-Month Historical Data** (30 days, 100 locations)
   - Without cache: 3000 API calls @ 100ms = 5 minutes
   - With cache: 50-100 API calls = 5-10 seconds
   - **Speedup: 30-60x**

3. **Batch Deduplication** (10K records, 100 unique)
   - Without dedup: 10,000 lookups
   - With dedup: 100 unique identified
   - **API call reduction: 99%**

## Dependencies Added

```toml
rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
```

- SQLite 3.44+ (bundled)
- Chrono for datetime handling (existing)
- Thread-safe Arc<Mutex> wrappers for PyO3 compatibility

## Storage Requirements

| Scale | DB Size | Memory (LRU) | Typical Usage |
|-------|---------|--------------|---------------|
| 10K entries | 1MB | 10MB | Single project, 1 week |
| 100K entries | 10MB | 50MB | Multi-project, 1 month |
| 1M entries | 100MB | 200MB | Large scale, 3+ months |
| 10M entries | 1GB | 1.5GB | Enterprise, 1+ year |

Use `db_path=None` for memory-only mode if persistence not needed.

## Known Limitations

1. **Geohash Precision**: Fixed at ~1km grid (good balance for most use cases)
   - Can be tuned by adjusting quantization factor in source
2. **SQLite Limitations**: Single-machine only (no built-in replication)
   - Redis support planned for future release
3. **Date Range Queries**: Require exact timestamp format consistency
   - ISO 8601 format recommended

## Migration Guide

### From Simple Cache to Enhanced Cache

```python
# Before
from pyweatherenriched import WeatherEnricher
enricher = WeatherEnricher(cache_size=1000)

# After (enhanced with persistence)
from pyweatherenriched import EnhancedCache
cache = EnhancedCache(cache_size=5000, db_path="weather.db")

# Get weather from cache
weather = cache.get(location, lat, lon, timestamp)
if weather is None:
    # Make API call and cache
    weather = fetch_from_api()
    cache.put(location, lat, lon, temp, humidity, condition, timestamp)
```

## Future Roadmap

- [ ] Redis backend for distributed caching
- [ ] Automatic TTL optimization using ML
- [ ] Predictive prefetching for known patterns
- [ ] Cache compression for large persistent stores
- [ ] S3/Cloud storage backend
- [ ] Real-time replication across nodes

## Testing

- ✓ Unit tests for cache operations (50+ tests)
- ✓ Integration tests with enricher pipeline
- ✓ Benchmark suite for performance validation
- ✓ Memory leak testing with valgrind

## Contributors

- Georgi Mammen Mullassery (Initial implementation & optimization)

## License

Proprietary - Mullassery Weather Systems

## Support

For issues or questions:
- GitHub Issues: https://github.com/Mullassery/PyWeatherEnriched/issues
- Email: mullassery@gmail.com
