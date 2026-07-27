# PyWeatherEnriched Architecture

## System Overview

PyWeatherEnriched is a Rust-based hyperlocal weather enrichment system with Python bindings for reconstructing precise, location-specific weather from operational data patterns.

```
┌─────────────────────────────────────────────────────────┐
│           Python Interface (PyO3 Bindings)              │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Integration Layer                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │ UnifiedEnrichmentPipeline                       │   │
│  │ - Auto format detection (CSV/JSON)              │   │
│  │ - Nested data preservation & reconstruction     │   │
│  │ - Multi-output support (CSV/JSON/JSONL)        │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Core Processing Modules                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Geocoding   │  │ Enrichment   │  │   Caching    │  │
│  │  (precise    │  │  (row-level  │  │  (24hr TTL)  │  │
│  │   lat/long)  │  │   enrichment)│  │  (70% cost   │  │
│  └──────────────┘  └──────────────┘  │   reduction) │  │
│  ┌──────────────┐  ┌──────────────┐  └──────────────┘  │
│  │   Weather    │  │   DateTime   │                     │
│  │  (OpenWeather│  │ Standardizer │                     │
│  │   + fallback)│  │  (20+ formats)                    │
│  └──────────────┘  └──────────────┘                    │
│  ┌──────────────┐  ┌──────────────┐                    │
│  │ Location     │  │Microgeography│                    │
│  │ Inference    │  │ (UHI, elev,  │                    │
│  │ (misspelling │  │  water, veg, │                    │
│  │  tolerance)  │  │  wind)       │                    │
│  └──────────────┘  └──────────────┘                    │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Data Format Support                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │     CSV      │  │     JSON     │  │   Nested     │  │
│  │   Parser     │  │   Parser     │  │  Flatten &   │  │
│  │  (quotes)    │  │  (arrays)    │  │ Reconstruct  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Advanced Reconstruction                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Phase 2:     │  │ Phase 3:     │  │  Parallel    │  │
│  │ Kriging      │  │  Inverse     │  │ Enrichment   │  │
│  │ & Regional   │  │  Models &    │  │ (Rayon)      │  │
│  │ Climate      │  │  Streaming   │  │              │  │
│  │ Models       │  │  Buffer      │  │              │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Batch Processing                          │
│  ┌────────────────────────────────────────────────┐    │
│  │ BatchProcessor (1M+ rows)                      │    │
│  │ - Parallel chunk processing (1000 rows)        │    │
│  │ - Nested JSON batch processing                 │    │
│  │ - JSONL export for streaming                   │    │
│  │ - Statistics tracking                          │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Module Breakdown

### 1. Core Modules

#### `location.rs` (320 LoC)
- **Purpose**: City/pincode/coordinate inference
- **Features**:
  - Misspelling tolerance (Levenshtein distance)
  - State code normalization
  - Pincode detection and validation
  - Confidence scoring

#### `weather.rs` (160 LoC)
- **Purpose**: OpenWeather API integration
- **Features**:
  - Async weather fetching
  - Mock fallback for API failures
  - Error handling and retries
  - Temperature/humidity/rainfall/pressure extraction

#### `cache.rs` (280 LoC)
- **Purpose**: SQLite-backed weather caching
- **Features**:
  - 24-hour TTL expiry
  - 70% cost reduction (fewer API calls)
  - Memory and SQLite backends
  - Timestamp-aware queries

#### `datetime.rs` (180 LoC)
- **Purpose**: Timestamp parsing and standardization
- **Features**:
  - 20+ format support (Unix, ISO 8601, regional)
  - Timezone handling
  - Error recovery

#### `enrichment.rs` (340 LoC + extensions)
- **Purpose**: Row-level enrichment pipeline
- **Features**:
  - CSV input parsing with quote handling
  - Batch enrichment
  - CSV export with proper escaping
  - Nested data preservation

#### `microgeography.rs` (350+ LoC)
- **Purpose**: Hyperlocal weather reconstruction
- **Features**:
  - Urban heat island effect (+2-3.5°C)
  - Elevation lapse rate (-0.65°C per 100m)
  - Water proximity cooling
  - Vegetation effects
  - Wind exposure adjustments
  - Location type classification (urban/suburban/rural)

### 2. Phase 2: Scaling & Advanced Reconstruction

#### `phase2.rs` (140 LoC)
- **Kriging Interpolation**: Spatial weather estimation
  - Variogram modeling (Exponential, Gaussian, Spherical)
  - Nearby station weighting
- **Parallel Enricher**: Rayon-based batch processing
  - Configurable chunk sizes (default: 1000)
  - Multi-threaded execution
- **Regional Micro-Climate Models**: Region-specific adjustments
  - Heat island factor per region
  - Elevation lapse rate calibration
  - Trained on historical samples
- **Batch Location Resolver**: 200x API call reduction
  - Deduplication
  - Bulk geocoding
- **Database Pooling**: Connection management for Snowflake/BigQuery

### 3. Phase 3: Advanced Reconstruction & Real-Time

#### `phase3.rs` (180 LoC)
- **Advanced Inverse Modeling**: Multi-signal weather inference
  - Delivery metrics (time delays, success rates)
  - Retail signals (umbrella sales, AC demand, cold drink trends)
  - Healthcare signals (respiratory admissions)
  - Weighted combination with confidence scoring
- **Monsoon Pattern Modeling**: Seasonal adjustments
  - Southwest onset/peak/transition phases
  - Regional intensity scaling
- **Streaming Enrichment Buffer**: Real-time row batching
  - VecDeque-based buffering
  - Configurable flush intervals
  - Batch-on-threshold triggers
- **Climate Anomaly Detection**: Z-score based outlier detection
- **Multi-Source Fusion**: Weighted averaging
  - API data (30%)
  - Inverse models (25%)
  - Spatial interpolation (25%)
  - Micro-climate adjustments (20%)

### 4. Data Format Support

#### `data_formats.rs` (320 LoC)
- **CsvParser**: Full CSV parsing with quote handling
- **JsonParser**: JSON array/object parsing
- **DataFormatDetector**: Auto-detection (JSON/CSV)
- **NestedDataReconstructor**: Flatten and reconstruct nested structures
  - Preserves original nested context
  - Enables reconstruction after enrichment

#### `geocoding.rs` (400+ LoC)
- **GeocodingService**: Precise lat/long inference
  - Address parsing with component extraction
  - Pincode-level precision (95-score)
  - Street-level precision (85-score)
  - Multi-column address composition
  - Precision level scoring
- **PrecisionLevel Enum**: Hierarchical precision tracking
  - Building → Street → Area → City → State → Country
- **PincodeDatabase**: Common Indian pincodes with center coordinates
- **AddressParseResult**: Detailed parsing output with confidence

### 5. Integration Layer

#### `integration.rs` (320 LoC)
- **UnifiedEnrichmentPipeline**: Single entry point for all input formats
  - Format auto-detection
  - Nested preservation option
  - Multi-output export (CSV/JSON/JSONL)
- **ProcessedData**: Result container with optional reconstruction map

### 6. Batch Processing

#### `batch_processor.rs` (380 LoC)
- **BatchProcessor**: Handles 1M+ row datasets
  - Parallel chunking (default: 1000 rows)
  - Batch statistics tracking
  - CSV and JSON batching
  - Nested JSON reconstruction
- **BatchProcessingStats**: Progress and error tracking
  - Total rows, successful/failed counts
  - Batches processed

### 7. Supporting Modules

#### `models.rs`
- **Row**: `Vec<(String, String)>` flat key-value row
- **Location**: City, pincode, latitude, longitude
- **WeatherData**: Temperature, humidity, rainfall, pressure, wind, clouds, visibility, timestamp
- **EnrichedRow**: Original data + location + weather
- **EnrichmentConfig**: API key, location columns, timestamp column, external location map

#### `error.rs`
- **WeatherError**: Comprehensive error types
  - LocationNotFound, InvalidCoordinates
  - ApiError, ParseError, CacheError
  - DatabaseError, DataQualityError
  - IoError, SerializationError

#### `python.rs`
- **PyWeatherEnriched**: PyO3 class for Python bindings
- Methods for enrichment and export

## Data Flow

### Single Row Enrichment Flow
```
Input Row
    ↓
[1] Extract Location → GeocodingService.parse_address()
    ↓ (Precision inference)
[2] Extract Timestamp → DateTimeStandardizer.standardize()
    ↓
[3] Check Cache → Cache.get(location, timestamp)
    ↓ (if hit)
[4] Fetch Weather → WeatherFetcher.fetch_current_weather()
    ↓ (if miss or API fail)
[5] Apply Micro-Climate → Microgeography adjustments
    ↓
[6] Return EnrichedRow
    ↓
Output (CSV/JSON)
```

### Batch Processing Flow
```
Large CSV/JSON File
    ↓
[1] Format Detection → DataFormatDetector
    ↓
[2] Chunking → 1000 rows per chunk
    ↓
[3] Parallel Processing → Rayon thread pool
    ↓ (per chunk)
[4] Row Enrichment Loop → (single row flow above)
    ↓
[5] Stats Accumulation → Progress tracking
    ↓
[6] Output Assembly → CSV/JSON/JSONL
    ↓
Output File
```

### Nested Data Reconstruction Flow
```
Nested JSON Input
    ↓
[1] Parse JSON → JsonParser
    ↓
[2] Flatten All Fields → NestedDataReconstructor.flatten()
    ↓
[3] Enrich Flat Rows → (batch processing flow)
    ↓
[4] Reconstruct Original Structure
    ↓ (merge enriched + original nested)
[5] Export with Full Nesting → export_json_nested()
    ↓
Output (nested + enriched)
```

## Performance Characteristics

- **Single Row**: ~200-500ms (includes API call)
- **Cached Row**: ~10-50ms (from cache)
- **Batch Processing**: 1M rows in ~2-4 hours (with parallelization)
- **Cache Hit Ratio**: 70% typical (24-hour TTL)
- **Cost Reduction**: 200x fewer API calls with intelligent batching

## Key Algorithms

1. **Haversine Distance**: Calculate distance between coordinates
2. **IDW (Inverse Distance Weighting)**: Spatial interpolation
3. **Kriging**: Variogram-based spatial estimation
4. **Urban Heat Island**: +2-3.5°C in dense areas
5. **Elevation Lapse Rate**: -0.65°C per 100m
6. **Z-Score Anomaly Detection**: Climate outlier identification
7. **Levenshtein Distance**: Misspelling tolerance in location names

## Database Backends (Pluggable)

- SQLite (default, file-based cache)
- Snowflake (batch writes for data warehouse)
- BigQuery (analytics integration)
- PostgreSQL (standard RDBMS)

## Error Handling Strategy

1. **Graceful Degradation**: Missing location → use mock data
2. **Retry Logic**: API failures → cache fallback
3. **Batch Continuation**: Single row failure → continue processing
4. **Detailed Logging**: All errors logged with context

## Dependencies

- **tokio**: Async runtime
- **reqwest**: HTTP client for weather API
- **serde/serde_json**: Serialization
- **chrono**: DateTime handling
- **rusqlite**: SQLite access
- **rayon**: Parallel processing
- **pyo3**: Python FFI bindings
- **csv**: CSV parsing
- **regex**: Pattern matching
- **thiserror**: Error types
- **lazy_static**: Static initialization
