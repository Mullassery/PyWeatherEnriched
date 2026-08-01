# Reverse Geocoding Guide

Convert latitude/longitude to postal codes, addresses, and administrative boundaries.

**Features:**
- ✅ Postal code lookup from coordinates
- ✅ Full address extraction (street, city, state, country)
- ✅ Administrative boundaries (county, neighborhood)
- ✅ Configurable output detail levels (Minimal → Complete)
- ✅ Multiple data sources with auto-detection
- ✅ LRU caching for performance
- ✅ Batch processing support

---

## Quick Start

### Python API

```python
from pyweatherenriched import ReverseGeocoder
from pyweatherenriched.geospatial import OutputDetailLevel

# Create reverse geocoder
geocoder = ReverseGeocoder(config_path="geospatial.toml")

# Single location - full result
result = geocoder.reverse_geocode(40.7128, -74.0060)
print(result['postal_code'])  # "10001"
print(result['city'])          # "New York"
print(result['state'])         # "NY"

# With configurable output detail level
result_minimal = geocoder.reverse_geocode_with_detail(
    latitude=40.7128,
    longitude=-74.0060,
    detail_level=OutputDetailLevel.Minimal
)
# {"postal_code": "10001", "postal_code_type": "ZIP", "country": "US", "confidence": 0.95}

result_standard = geocoder.reverse_geocode_with_detail(
    latitude=40.7128,
    longitude=-74.0060,
    detail_level=OutputDetailLevel.Standard
)
# Includes street_address, city, state, country_code, source

result_extended = geocoder.reverse_geocode_with_detail(
    latitude=40.7128,
    longitude=-74.0060,
    detail_level=OutputDetailLevel.Extended
)
# Includes admin_level_1 (state), admin_level_2 (county), neighborhood

result_complete = geocoder.reverse_geocode_with_detail(
    latitude=40.7128,
    longitude=-74.0060,
    detail_level=OutputDetailLevel.Complete
)
# Includes alternatives from other sources, sources_tried, processing_time_ms
```

---

## Output Formats

### Minimal
```json
{
  "postal_code": "10001",
  "postal_code_type": "ZIP",
  "country": "US",
  "confidence": 0.95
}
```

### Standard
```json
{
  "postal_code": "10001",
  "postal_code_type": "ZIP",
  "street_address": "350 5th Ave",
  "city": "New York",
  "state": "NY",
  "country": "US",
  "country_code": "US",
  "confidence": 0.95,
  "source": "osm"
}
```

### Extended
```json
{
  "postal_code": "10001",
  "postal_code_type": "ZIP",
  "street_address": "350 5th Ave",
  "city": "New York",
  "state": "NY",
  "country": "US",
  "country_code": "US",
  "admin_level_1": "New York",
  "admin_level_2": "New York County",
  "neighborhood": "Koreatown",
  "confidence": 0.95,
  "source": "osm"
}
```

### Complete
```json
{
  "primary": {
    "postal_code": "10001",
    "postal_code_type": "ZIP",
    "street_address": "350 5th Ave",
    "city": "New York",
    "state": "NY",
    "country": "US",
    "country_code": "US",
    "admin_level_1": "New York",
    "admin_level_2": "New York County",
    "neighborhood": "Koreatown",
    "latitude": 40.7128,
    "longitude": -74.0060,
    "confidence": 0.95,
    "source": "osm"
  },
  "alternatives": [
    {
      "postal_code": "10002",
      "postal_code_type": "ZIP",
      "confidence": 0.70,
      "source": "google"
    },
    {
      "postal_code": "10003",
      "postal_code_type": "ZIP",
      "confidence": 0.65,
      "source": "usps"
    }
  ],
  "sources_tried": ["osm", "google", "usps"],
  "processing_time_ms": 45
}
```

---

## Configuration

### OpenStreetMap (CRITICAL - Always Available)
```toml
[reverse_geocoding.osm]
source = "local_file"
base_path = "/data/geospatial"
file_pattern = "osm/{lat}_{lon}.geojson"
cache_enabled = true
cache_ttl_seconds = 2592000  # 30 days
```

### Google Maps (Optional)
```toml
[reverse_geocoding.google]
enabled = true
api_key = "YOUR_GOOGLE_MAPS_API_KEY"
cache_enabled = true
cache_ttl_seconds = 2592000
```

### USPS Postal Database (Optional)
```toml
[reverse_geocoding.usps]
enabled = true
database_path = "/data/usps_postal_codes.db"
cache_enabled = true
cache_ttl_seconds = 2592000
```

### Hybrid (Auto-Detect)
```toml
[reverse_geocoding]
sources = ["osm", "google", "usps"]
priority_order = ["osm", "google", "usps"]  # Try in this order
fallback_enabled = true  # If OSM fails, try Google
timeout_seconds = 10
```

---

## Batch Processing

### Process Multiple Locations

```python
from pyweatherenriched.geospatial import BatchReverseGeocoder

geocoder = ReverseGeocoder(config_path="geospatial.toml")
batch = BatchReverseGeocoder(geocoder)

# Coordinates with identifiers
locations = [
    (40.7128, -74.0060, "delivery_001"),
    (34.0522, -118.2437, "delivery_002"),
    (41.8781, -87.6298, "delivery_003"),
]

results = batch.process_with_progress(locations)

for result in results:
    if result['error']:
        print(f"{result['identifier']}: Error - {result['error']}")
    else:
        print(f"{result['identifier']}: {result['result']['postal_code']}")
```

### Filter by Postal Code Pattern

```python
results = batch.process_with_progress(locations)

# Get all results with specific postal code range
ny_results = [
    r for r in results
    if r['result'] and r['result']['postal_code'].startswith('10')
]

print(f"Found {len(ny_results)} deliveries in Manhattan")
```

---

## Performance Optimization

### Caching

```python
# Cache is enabled by default for frequently looked-up locations
geocoder = ReverseGeocoder(config_path="geospatial.toml", cache_enabled=True)

# Check cache statistics
hits, size = geocoder.cache_stats()
print(f"Cache: {hits} hits, {size} entries")

# Clear cache if needed
geocoder.clear_cache()
```

### Batch Processing Strategy

```python
# For large batches, use parallel processing with batches
from concurrent.futures import ThreadPoolExecutor

def geocode_location(geocoder, lat, lon, identifier):
    try:
        result = geocoder.reverse_geocode(lat, lon)
        return {'identifier': identifier, 'result': result, 'error': None}
    except Exception as e:
        return {'identifier': identifier, 'result': None, 'error': str(e)}

locations = [(lat, lon, id) for lat, lon, id in ...]

with ThreadPoolExecutor(max_workers=4) as executor:
    results = list(executor.map(
        lambda l: geocode_location(geocoder, l[0], l[1], l[2]),
        locations
    ))
```

---

## Integration with Weather Enrichment

```python
from pyweatherenriched import WeatherEnricher, ReverseGeocoder

weather_enricher = WeatherEnricher(cache=distributed_cache)
geocoder = ReverseGeocoder(config_path="geospatial.toml")

# Enrich with both weather AND reverse geocoding
result = {
    **weather_enricher.enrich("New York", "2024-01-15T12:00:00Z"),
    **geocoder.reverse_geocode(40.7128, -74.0060)
}

print(result)
# {
#   'temperature': 15.0,
#   'humidity': 65.0,
#   'postal_code': '10001',
#   'city': 'New York',
#   'state': 'NY',
#   ...
# }
```

---

## Auto-Detection Behavior

The system automatically detects available data sources and uses them intelligently:

```
User requests reverse geocoding at (40.7128, -74.0060)
    ↓
Check available sources
    ├─ OSM data available? → Try first (fastest, no API costs)
    ├─ Google Maps API configured? → Try if OSM returns low confidence
    └─ USPS database available? → Use as fallback
    ↓
Return best match with confidence score
```

**Priority (by default):**
1. **OpenStreetMap (CRITICAL)** - Always available, no API cost
2. **Google Maps (Optional)** - Higher accuracy, API cost
3. **USPS Database (Optional)** - US-only, comprehensive
4. **Custom sources** - User-defined

---

## Data Preparation

### OSM Data
```bash
# Extract postal code boundaries from OSM
ogr2ogr -f GeoJSON \
    -where "postal_code IS NOT NULL" \
    postal_codes.geojson \
    osm_data.osm

# Convert to GeoJSON tiles by location
# Store as: /data/osm/{lat}_{lon}.geojson
```

### USPS Database
```bash
# Download USPS postal database
# Convert to SQLite: usps_postal_codes.db
# Fields: postal_code, latitude, longitude, city, state

# Create indices for fast lookup
CREATE INDEX idx_latlon ON postal_codes(latitude, longitude);
CREATE INDEX idx_postal ON postal_codes(postal_code);
```

---

## Error Handling

```python
from pyweatherenriched.geospatial import ReverseGeocodingError

try:
    result = geocoder.reverse_geocode(40.7128, -74.0060)
except ReverseGeocodingError as e:
    print(f"Reverse geocoding failed: {e}")
    # Fall back to approximate postal code or skip enrichment
```

---

## Future Enhancements

When implementing optional reverse geocoding sources:

**Google Maps Implementation:**
- API key configuration
- Rate limiting (500 requests/second)
- Fallback to OSM if rate limited
- Confidence scoring

**USPS Database Implementation:**
- SQLite lookup by proximity
- Nearest postal code matching
- ZIP+4 extended postal codes
- Address matching algorithm

**Custom Sources:**
- Support for user-provided geocoding services
- Custom confidence scoring
- Custom output formats

---

## Support & Contributing

To add a new reverse geocoding source:
1. Create implementation following GoogleMapsReverseGeocoder pattern
2. Add configuration support in config.rs
3. Update auto-detection logic
4. Add tests and documentation
5. Submit PR

See contributing guide for details.
