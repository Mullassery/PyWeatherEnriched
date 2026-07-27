# Phase 2: Scaling Architecture (3M+ Rows)

**Priority**: 🔴 CRITICAL - Design for scale from day one

---

## Performance Targets

| Benchmark | Current (P1) | Phase 2 Target | 3M Rows Time |
|-----------|-------------|---|---|
| Throughput | 100K rows/30s | 1M rows/60s | ~3 minutes |
| Memory/1M rows | ~500MB | ~200MB | Streaming |
| API calls/1M rows | 1M (worst case) | ~5K (batched) | Cached |
| Total cost/3M rows | ~$1,500 | ~$15 | 100x reduction |

---

## Scaling Strategy: 3-Tier Approach

### Tier 1: Multi-Core Parallelization (Rayon)
```
Input: 3M rows
      ↓
[Partition into N chunks] (4-16 chunks per CPU core)
      ↓
[Parallel enrichment] (4-16 threads)
      ├─ Thread 1: Rows 1-200K
      ├─ Thread 2: Rows 200K-400K
      ├─ Thread 3: Rows 400K-600K
      └─ Thread N: Rows 2.8M-3M
      ↓
[Merge results]
      ↓
Output: 3M enriched rows

Expected: 4-8x speedup on 8-core machine
```

### Tier 2: Batch Location Resolution
```
Instead of:
  Row 1: Location="Mumbai" → API call 1
  Row 2: Location="Mumbai" → API call 2 (duplicate!)
  Row 3: Location="Mumbai" → API call 3 (duplicate!)
  ...

Do this:
  Scan all rows → Find unique locations
  Mumbai appears 50K times → Make 1 API call, cache result
  Bangalore appears 30K times → Make 1 API call, cache result
  ...
  
Total: 5K unique locations → 5K API calls (vs. 1M!)
Result: 200x API call reduction
```

### Tier 3: Streaming I/O
```
Read → Process → Write (in parallel pipelines)

Instead of:
  [Load 3M rows] → [Process 3M rows] → [Write 3M rows]
  (Memory spike: 3M rows × 200 bytes = 600MB+)

Do this:
  [Read chunk 1 (10K rows)]
         ↓
  [Enrich chunk 1 (parallel)]
         ↓
  [Write chunk 1] ← [Read chunk 2 in parallel]
         ↓
  [Enrich chunk 2 (parallel)]
         ↓
  [Write chunk 2] ← [Read chunk 3 in parallel]

Result: Constant memory usage (~50MB for chunk)
```

---

## Implementation: Phase 2.1 Details

### 1. Parallel Enrichment (Rayon)

```rust
use rayon::prelude::*;

pub async fn enrich_batch_parallel(&self, rows: Vec<Vec<(String, String)>>) -> Result<Vec<EnrichedRow>> {
    // Partition into chunks (auto-determined by Rayon)
    rows.par_chunks(10_000)  // 10K rows per chunk
        .flat_map_iter(|chunk| {
            // Each thread enriches its chunk
            chunk.iter().map(|row| self.enrich_row_sync(row))
        })
        .collect::<Result<Vec<_>>>()
}
```

**Expected performance**:
- 1M rows: ~60 seconds (16.7K rows/sec) on 8-core
- 3M rows: ~3 minutes

### 2. Batch Location Resolution

```rust
pub fn batch_resolve_locations(&self, rows: &[Vec<(String, String)>]) -> Result<HashMap<String, Location>> {
    // Extract unique locations
    let mut unique_locations = HashSet::new();
    for row in rows {
        if let Ok(loc_str) = self.extract_location_string(row) {
            unique_locations.insert(loc_str);
        }
    }

    // Resolve in parallel
    let mut location_cache = HashMap::new();
    for loc_str in unique_locations {
        if let Ok(location) = LocationInference::detect_location(&loc_str) {
            location_cache.insert(loc_str, location);
        }
    }

    Ok(location_cache)
}
```

**Expected performance**:
- 1M rows → ~5K unique locations
- Local resolution: <100ms
- API calls: 5K (vs. 1M without batching)

### 3. Streaming CSV Reader

```rust
use csv::ReaderBuilder;

pub async fn enrich_csv_stream(
    &self,
    input_path: &str,
    output_path: &str,
    chunk_size: usize,
) -> Result<()> {
    let reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(input_path)?;

    let mut writer = csv::Writer::from_path(output_path)?;

    // Process in chunks
    let mut chunk = Vec::with_capacity(chunk_size);
    
    for result in reader.into_records() {
        let record = result?;
        chunk.push(record);

        if chunk.len() >= chunk_size {
            // Process chunk in parallel
            self.enrich_batch_parallel(chunk).await?;
            chunk = Vec::with_capacity(chunk_size);
        }
    }

    // Process remaining
    if !chunk.is_empty() {
        self.enrich_batch_parallel(chunk).await?;
    }

    Ok(())
}
```

**Memory usage**: ~50-100MB (fixed, not growing with input size)

---

## Database Integration: Streaming Writes

For 3M rows → Snowflake/BigQuery/Postgres:

```rust
pub async fn enrich_and_stream_to_db(
    &self,
    input: &str,
    db_conn: &Connection,
    chunk_size: usize,
) -> Result<()> {
    let enriched_chunks = self.enrich_csv_stream_chunks(input, chunk_size)?;

    // Stream writes with connection pooling
    for chunk in enriched_chunks {
        // Batch insert 10K rows at a time
        db_conn.batch_insert(&chunk).await?;
    }

    Ok(())
}
```

**Expected performance**:
- Snowflake: 10K rows/sec (5 minutes for 3M)
- BigQuery: 20K rows/sec (2.5 minutes for 3M)
- Postgres: 5K rows/sec (10 minutes for 3M)

---

## Caching Strategy at Scale

### Multi-Level Cache

```
L1: In-Memory Cache (Thread-local, 10K entries)
      ↓ (miss)
L2: Redis Cache (Shared, 1M entries, 24h TTL)
      ↓ (miss)
L3: SQLite Cache (Persistent, unlimited, 30d TTL)
      ↓ (miss)
L4: API Call (OpenWeather)
```

**Expected cache hit rates**:
- Same location, same date: 95%+
- Different date, same location: 70-80%
- Overall: 80-90% (reduces API calls by 5-10x)

---

## Error Recovery & Resumption

For long-running enrichment (3M rows × 2 hours):

```rust
pub struct EnrichmentCheckpoint {
    total_rows: usize,
    processed_rows: usize,
    failed_rows: usize,
    last_checkpoint: DateTime<Utc>,
    checkpoint_file: String,
}

impl EnrichmentCheckpoint {
    pub fn save(&self) -> Result<()> {
        // Save progress to checkpoint file every 10K rows
        let json = serde_json::to_string(self)?;
        std::fs::write(&self.checkpoint_file, json)?;
        Ok(())
    }

    pub fn resume(checkpoint_file: &str) -> Result<Self> {
        // Resume from last checkpoint if process crashes
        let json = std::fs::read_to_string(checkpoint_file)?;
        serde_json::from_str(&json).map_err(|e| WeatherError::SerializationError(e.to_string()))
    }
}
```

**Feature**: Resume 3M-row enrichment from last checkpoint (no re-processing)

---

## Monitoring & Progress Tracking

```rust
pub struct EnrichmentMetrics {
    total_rows: usize,
    processed_rows: usize,
    failed_rows: usize,
    cache_hits: usize,
    cache_misses: usize,
    api_calls: usize,
    elapsed_time: Duration,
}

impl EnrichmentMetrics {
    pub fn throughput(&self) -> f64 {
        self.processed_rows as f64 / self.elapsed_time.as_secs_f64()
    }

    pub fn cache_hit_rate(&self) -> f64 {
        self.cache_hits as f64 / (self.cache_hits + self.cache_misses) as f64
    }

    pub fn estimated_remaining_time(&self) -> Duration {
        let remaining_rows = self.total_rows - self.processed_rows;
        let remaining_seconds = remaining_rows as f64 / self.throughput();
        Duration::from_secs_f64(remaining_seconds)
    }
}
```

**Output for 3M-row enrichment**:
```
PyWeatherEnriched Enrichment Progress
=====================================
Total rows: 3,000,000
Processed: 1,500,000 (50%)
Failed: 15 (0.001%)
Cache hit rate: 87.3%
Throughput: 12,500 rows/sec
Elapsed: 2m 0s
Estimated remaining: 2m 0s
API calls made: 3,847
API cost: $5.77
```

---

## Phase 2.1 Milestones

### Week 1: Parallelization
- [ ] Integrate Rayon for parallel enrichment
- [ ] Batch location resolution
- [ ] Benchmark: 1M rows/60s target

### Week 2: Streaming & I/O
- [ ] Streaming CSV reader
- [ ] Streaming DB writes
- [ ] Memory profiling (<200MB for 3M rows)

### Week 3: Error Recovery
- [ ] Checkpoint system
- [ ] Resume capability
- [ ] Progress tracking

### Week 4: Optimization
- [ ] Multi-level caching (Redis, SQLite)
- [ ] Connection pooling
- [ ] Final benchmarking

---

## Estimated Costs: 3M Rows

### Phase 1 (Sequential, no cache)
```
3M rows × 0.9 unique locations = ~2.7M API calls
2.7M calls × $0.0015/call = $4,050
Time: ~7 hours
```

### Phase 2 (Batched, cached)
```
2.7M calls × batch reduction (5x) = 540K calls
540K calls × 70% cache hit = 162K actual API calls
162K calls × $0.0015/call = $243
Time: ~3 minutes
Cost savings: 94% reduction ($4,050 → $243)
```

---

## Next Steps

1. **Phase 2.1** (4 weeks): Implement parallelization + streaming
2. **Phase 2.2** (2 weeks): External location mapping + reverse geocoding
3. **Phase 2.3** (2 weeks): Database integrations
4. **Phase 2.4** (2 weeks): NoSQL support
5. **Beta** (1 week): Load testing with 10M-row dataset
6. **GA** (1 week): Production launch

---

## Conclusion

Scaling to 3M+ rows requires:
1. **Parallelization** (Rayon): 4-8x speedup
2. **Batching** (unique locations): 200x API call reduction  
3. **Streaming I/O** (chunks): Constant memory usage
4. **Caching** (L2/L3): 70-90% cache hit rate
5. **Error recovery** (checkpoints): Resumable long runs

**Target**: 3M rows in 3 minutes, <$250 cost

---

Status: 📋 READY FOR PHASE 2.1  
Priority: 🔴 CRITICAL (3M-row support is requirement)
