# Data Formats & Input Processing Guide

PyWeatherEnriched now supports multiple input formats with automatic detection, nested data reconstruction, and multi-column address parsing for precise geocoding.

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

```rust
use pyweatherenriched::{UnifiedEnrichmentPipeline, EnrichmentConfig};

#[tokio::main]
async fn main() {
    let config = EnrichmentConfig::new(
        "your_api_key".to_string(),
        vec!["location".to_string()],
        "timestamp".to_string(),
    );

    let pipeline = UnifiedEnrichmentPipeline::new(config).unwrap();

    let csv = r#"location,timestamp,sales
Mumbai,2025-06-15T10:00:00Z,5000
Delhi,2025-06-15T11:00:00Z,3000"#;

    let result = pipeline.process_csv(csv).await.unwrap();
    let csv_output = pipeline.export_csv(&result.into()).unwrap();
    println!("{}", csv_output);
}
```

### Example 2: JSON Processing with Nested Reconstruction

```rust
use pyweatherenriched::{UnifiedEnrichmentPipeline, EnrichmentConfig};

#[tokio::main]
async fn main() {
    let config = EnrichmentConfig::new(
        "your_api_key".to_string(),
        vec!["location".to_string()],
        "timestamp".to_string(),
    );

    let pipeline = UnifiedEnrichmentPipeline::new(config).unwrap();

    let json = r#"[{
        "order_id": "ORD-001",
        "delivery": {
            "location": "Mumbai",
            "timestamp": "2025-06-15T10:00:00Z"
        }
    }]"#;

    let result = pipeline.process_json_nested(json).await.unwrap();
    let json_output = pipeline.export_json_nested(&result).unwrap();
    println!("{}", json_output);
}
```

### Example 3: Multi-Column Address Geocoding

```rust
use pyweatherenriched::{GeocodingService, EnrichmentConfig, Enricher};

#[tokio::main]
async fn main() {
    let geocoder = GeocodingService::new();

    // From row data
    let row = vec![
        ("street".to_string(), "123 Main Street".to_string()),
        ("city".to_string(), "Mumbai".to_string()),
        ("state".to_string(), "Maharashtra".to_string()),
        ("pincode".to_string(), "400001".to_string()),
    ];

    let location = geocoder.compose_from_row(&row, &[]).unwrap();
    println!("Latitude: {}, Longitude: {}", location.latitude, location.longitude);
}
```

### Example 4: Batch Processing Large Datasets

```rust
use pyweatherenriched::{BatchProcessor, EnrichmentConfig};

#[tokio::main]
async fn main() {
    let config = EnrichmentConfig::new(
        "your_api_key".to_string(),
        vec!["location".to_string()],
        "timestamp".to_string(),
    );

    let processor = BatchProcessor::new(config).unwrap();

    let csv = "location,timestamp,sales\n..."; // 1M+ rows

    let result = processor.process_csv_batches(csv).await.unwrap();
    
    println!("Processed: {} rows", result.stats.total_rows);
    println!("Successful: {}", result.stats.successful_enrichments);
    println!("Failed: {}", result.stats.failed_enrichments);
    
    let csv_output = processor.export_csv(&result).unwrap();
}
```

## Automatic Format Detection

The `DataFormatDetector` automatically identifies input format:

```rust
use pyweatherenriched::{DataFormatDetector};

let content = "..."; // CSV or JSON

let rows = DataFormatDetector::detect_and_parse(content).unwrap();
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
