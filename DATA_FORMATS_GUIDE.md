# Data Formats & Input Processing Guide

PyWeatherEnriched now supports multiple input formats with automatic detection, nested data reconstruction, and multi-column address parsing for precise geocoding.

> ⚡ **No API keys needed!** The library works completely out of the box. All examples below require zero configuration.

## Supported Input Formats

### 1. CSV (Comma-Separated Values)
Simple tabular data with headers.

```csv
order_id,location,timestamp,sales
ORD-001,Mumbai,2025-06-15T10:00:00Z,5000
ORD-002,Delhi,2025-06-15T11:00:00Z,3000
ORD-003,Bangalore,2025-06-15T12:00:00Z,4500
```

**Features:**
- Quote handling for fields containing commas
- Escaped quotes within quoted fields (`"He said ""hello"""`)
- Automatic header detection

### 2. JSON (Single Objects or Arrays)
Structured data with nested support.

```json
[
  {
    "order_id": "ORD-001",
    "location": "Mumbai",
    "delivery": {
      "timestamp": "2025-06-15T10:00:00Z",
      "address": {
        "street": "123 Main St",
        "pincode": "400001"
      }
    },
    "sales": 5000
  }
]
```

**Features:**
- Automatic nested data flattening for enrichment
- Preservation of original nested structure
- Reconstruction of nested output after enrichment

### 3. Multi-Column Address Data
Address split across multiple columns for precise geocoding.

```csv
order_id,street,city,state,pincode,timestamp
ORD-001,123 Main Street,Mumbai,Maharashtra,400001,2025-06-15T10:00:00Z
ORD-002,456 King Road,Delhi,Delhi,110001,2025-06-15T11:00:00Z
```

**Features:**
- Automatic column detection (street/address, city, state, pincode/postal)
- Component-level geocoding for building-level precision
- Fallback chain: pincode → street+city → city-only

## Usage Examples

### Example 1: Simple CSV Processing

```python
from pyweatherenriched import enricher

# No API key needed!
csv = """location,timestamp,sales
Mumbai,2025-06-15T10:00:00Z,5000
Delhi,2025-06-15T11:00:00Z,3000"""

result = enricher.process_csv(
    csv_content=csv,
    location_column='location',
    timestamp_column='timestamp'
)

csv_output = enricher.export_csv(result)
print(csv_output)
```

### Example 2: JSON Processing with Nested Reconstruction

```python
from pyweatherenriched import enricher

json_data = """[{
    "order_id": "ORD-1",
    "delivery": {
        "location": "Mumbai",
        "timestamp": "2025-06-15T10:00:00Z"
    }
}]"""

result = enricher.process_json(
    json_content=json_data,
    preserve_nesting=True
)

json_output = enricher.export_json_nested(result)
print(json_output)
```

### Example 3: Multi-Column Address Geocoding

```python
from pyweatherenriched import geocoder

# Multi-column address (no API call needed!)
row = [
    ("street", "123 Main Street"),
    ("city", "Mumbai"),
    ("state", "Maharashtra"),
    ("pincode", "400001"),
]

location = geocoder.compose_from_row(row, [])
print(f"Latitude: {location.latitude}, Longitude: {location.longitude}")
```

### Example 4: Batch Processing Large Datasets

```python
from pyweatherenriched import batch_processor

# Process 1M+ rows with parallel chunking (no API key needed!)
result = batch_processor.process_csv_batches(
    csv_content=csv_data,  # CSV string
    location_column='location',
    timestamp_column='timestamp'
)

print(f"Processed: {result.stats.total_rows} rows")
print(f"Successful: {result.stats.successful_enrichments}")
print(f"Failed: {result.stats.failed_enrichments}")

csv_output = batch_processor.export_csv(result)
```

## Automatic Format Detection

The library automatically identifies input format:

```python
from pyweatherenriched import enricher

# CSV or JSON - library auto-detects!
result = enricher.process(content)  # No format specification needed
```

Detection logic:
1. **JSON**: Starts with `[` or `{`
2. **CSV**: Contains `,` or `\t` separators
3. **Error**: Cannot determine format

## Precision Levels in Geocoding

When parsing addresses, PyWeatherEnriched assigns precision scores:

- **Building** (95): Street number + street name + city + pincode
- **Street** (85): Street name + city + pincode
- **Area** (75): Neighborhood/area + city + pincode
- **City** (60): City name only
- **State** (40): State/region level
- **Country** (10): Country level only

Higher precision leads to better weather reconstruction. Use `precision_level` field in `AddressParseResult` to assess data quality:

```rust
let result = geocoder.parse_address("123 Main St, Mumbai, 400001").unwrap();
match result.precision_level {
    PrecisionLevel::Building => println!("High precision!"),
    PrecisionLevel::City => println!("City-level precision"),
    _ => println!("Lower precision")
}
```

## Nested Data Reconstruction

When processing nested JSON, PyWeatherEnriched:

1. **Flattens** all nested fields for enrichment
2. **Preserves** original structure information
3. **Reconstructs** output with both flat enriched data AND original nested structure

Example flow:

Input:
```json
{
  "order_id": "ORD-1",
  "delivery": {
    "location": "Mumbai",
    "timestamp": "2025-06-15T10:00:00Z"
  }
}
```

After enrichment (reconstructed):
```json
{
  "order_id": "ORD-1",
  "delivery": {
    "location": "Mumbai",
    "timestamp": "2025-06-15T10:00:00Z"
  },
  "enriched_weather": {
    "temperature": 32.5,
    "humidity": 78,
    "rainfall": 2.1,
    ...
  }
}
```

## Performance Optimization

For large datasets (1M+ rows):
- Use `BatchProcessor` with parallel chunking (default: 1000 rows/chunk)
- Process CSV files directly (faster than JSON)
- Enable batch writes to databases for optimal throughput

## Export Formats

### CSV Export
- All original columns + weather columns
- Flattened nested data (underscore-separated)
- Proper quote escaping for fields with commas

### JSON Export
- Reconstructed nested structure
- Original data preserved
- Weather data as additional fields

### JSONL Export (JSON Lines)
- One object per line
- Optimized for streaming/large datasets
- Suitable for newline-delimited JSON processors

## Error Handling

Common errors and solutions:

| Error | Cause | Solution |
|-------|-------|----------|
| `LocationNotFound` | No city/pincode found | Provide city or pincode column |
| `FormatError` | Invalid CSV/JSON | Check syntax, ensure headers present |
| `InvalidCoordinates` | Lat/long out of range | Verify geocoding results |
| `MissingColumn` | Required column not found | Check column name spelling |

## Best Practices

1. **Address Data**: Provide pincode when available (highest precision)
2. **Timestamps**: Use ISO 8601 format (2025-06-15T10:00:00Z)
3. **Large Files**: Use BatchProcessor with appropriate chunk sizes
4. **Nested Data**: Use JSON format to preserve structure context
5. **Validation**: Check `precision_level` in geocoding results
