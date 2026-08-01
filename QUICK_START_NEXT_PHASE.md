# Quick Start: Phase 2 Implementation (v0.4.0 - Redis Backend)

## 📋 One-Page Implementation Guide

### Goal
Add Redis support for distributed caching, enabling multi-service deployments and 40-60% additional API cost reduction.

### Timeline
**7 weeks, 2 engineers, $50K engineering cost**

---

## Week 1-2: Core Implementation

### Step 1: Add Dependencies
```toml
# Cargo.toml
redis = "0.25"
```

### Step 2: Create RedisCache Backend
```rust
// src/redis_cache.rs
use redis::Commands;
use crate::types::EnrichedData;

pub struct RedisCache {
    connection: redis::Connection,
    ttl_seconds: u32,
    proximity_radius_km: f64,
}

impl RedisCache {
    pub fn new(redis_url: &str, ttl_hours: u32) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_connection()?;
        Ok(RedisCache {
            connection,
            ttl_seconds: ttl_hours * 3600,
            proximity_radius_km: 5.0,
        })
    }

    pub fn get(&mut self, location: &str, lat: f64, lon: f64, ts: &str) 
        -> Option<EnrichedData> 
    {
        let key = format!("weather:{}:{}:{}", location, lat, lon);
        self.connection.get_ex::<_, String>(&key, 
            redis::SetOptions::default()
                .get()
        ).ok().and_then(|json| {
            EnrichedData::from_json(&json).ok()
        })
    }

    pub fn put(&mut self, data: EnrichedData) -> Result<()> {
        let key = format!("weather:{}:{}:{}", data.location, data.latitude, data.longitude);
        let json = data.to_json()?;
        self.connection.set_ex(&key, json, self.ttl_seconds)?;
        Ok(())
    }
}
```

### Step 3: Backend Trait Pattern
```rust
// src/cache_backend.rs
pub trait CacheBackend {
    fn get(&mut self, location: &str, lat: f64, lon: f64, ts: &str) -> Option<EnrichedData>;
    fn put(&mut self, data: EnrichedData) -> Result<()>;
    fn deduplicate_batch(&mut self, batch: &[(String, f64, f64, String)]) 
        -> (Vec<usize>, usize);
}

// Implement for both SQLiteCache and RedisCache
impl CacheBackend for RedisCache { ... }
impl CacheBackend for EnhancedCache { ... }
```

### Step 4: Python Bindings
```python
# New class: DistributedCache
from pyweatherenriched import DistributedCache

cache = DistributedCache(
    backend='redis',
    redis_url='redis://localhost:6379',
    ttl=72,
    proximity_radius=10.0
)

# Same API as EnhancedCache
result = cache.get(location, lat, lon, timestamp)
cache.put(location, lat, lon, temp, humidity, condition, timestamp)
```

---

## Week 3: Advanced Features

### Redis Cluster Support
```rust
pub struct RedisClusterCache {
    cluster: redis::cluster::ClusterConnection,
}

// Automatic failover, replication, sharding
```

### Pub/Sub for Cache Invalidation
```rust
// Publish invalidation events
pub.publish("weather:invalidate", json!({
    "location": location,
    "reason": "api_updated"
}))?;

// Subscribe and auto-clear stale entries
subscriber.on("weather:invalidate", |msg| {
    cache.invalidate(&msg.location)?;
})?;
```

### Redis Streams for Event Log
```rust
// Log all cache operations (audit trail)
stream::xadd("weather:operations", "*", &[
    ("op", "put"),
    ("location", location),
    ("lat", lat),
    ("lon", lon),
    ("timestamp", now),
])?;
```

---

## Week 4: Testing & Benchmarks

### Performance Comparison
```python
import time
from pyweatherenriched import EnhancedCache, DistributedCache

# Setup
sqlite_cache = EnhancedCache(cache_size=5000, db_path="test.db")
redis_cache = DistributedCache(
    backend='redis',
    redis_url='redis://localhost:6379'
)

# Benchmark: 10K gets
for cache in [sqlite_cache, redis_cache]:
    start = time.time()
    for i in range(10000):
        cache.get(f"location_{i}", 40.0 + i*0.001, -74.0, "2024-01-15")
    elapsed = time.time() - start
    print(f"{cache.__class__.__name__}: {10000/elapsed:.0f} ops/sec")
```

**Expected Results**:
- SQLite: 10K-20K ops/sec
- Redis (local): 50K-100K ops/sec
- Redis (cluster): 200K-500K ops/sec

### Multi-Service Test
```python
# Service A
cache_shared = DistributedCache(
    backend='redis',
    redis_url='redis://redis-cluster:6379'
)

# Service B
cache_shared = DistributedCache(
    backend='redis',
    redis_url='redis://redis-cluster:6379'
)

# Service A puts data
cache_shared.put('NY', 40.7128, -74.0060, 15.2, 65.0, 'Cloudy', '2024-01-15')

# Service B retrieves (cross-service!)
result = cache_shared.get('NY', 40.7128, -74.0060, '2024-01-15')
assert result is not None  # ✅
```

---

## Week 5: Documentation

### README Section
```markdown
## Distributed Caching with Redis

PyWeatherEnriched v0.4.0 supports distributed Redis caching for 
multi-service deployments.

### Quick Start
\`\`\`python
from pyweatherenriched import DistributedCache

cache = DistributedCache(
    backend='redis',
    redis_url='redis://redis:6379',
    ttl=72,
    proximity_radius=10.0
)

result = cache.get('NYC', 40.7128, -74.0060, '2024-01-15')
\`\`\`

### Architecture
- Memory efficiency: ~1KB per entry
- Latency: <5ms p50, <20ms p99
- Throughput: 50K+ ops/sec per Redis node
- Failover: Automatic with Redis Cluster
\`\`\`
```

### Example: Docker Compose
```yaml
# docker-compose.yml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports: [6379:6379]
    volumes: [redis_data:/data]
    command: redis-server --appendonly yes

  app:
    build: .
    ports: [8080:8080]
    depends_on: [redis]
    environment:
      - REDIS_URL=redis://redis:6379
    command: python -m pyweatherenriched.server

volumes:
  redis_data:
```

### Example: Multi-Service Setup
```python
# service_a.py - Delivery ETA service
cache = DistributedCache(backend='redis', redis_url='redis://redis:6379')
enricher = WeatherEnricher(cache=cache)

@app.get("/eta/{delivery_id}")
def get_eta(delivery_id):
    delivery = get_delivery(delivery_id)
    weather = enricher.enrich(delivery.location, delivery.timestamp)
    return adjust_eta(weather)

# service_b.py - Demand forecasting service
cache = DistributedCache(backend='redis', redis_url='redis://redis:6379')
enricher = WeatherEnricher(cache=cache)

@app.get("/forecast/{region}")
def forecast(region):
    # Reuses cache from service_a!
    weather = enricher.enrich_batch(region_locations, timestamp)
    return predict_demand(weather)
```

---

## Week 6: Release Preparation

### Version Bump
```toml
# Cargo.toml
version = "0.4.0"
```

### Release Notes
```markdown
# v0.4.0 - Distributed Redis Caching

## What's New
- ✅ Redis backend for distributed caching
- ✅ Multi-service cache sharing
- ✅ Redis Cluster support (HA, automatic failover)
- ✅ Sub-5ms latency, 50K+ ops/sec throughput
- ✅ Pub/Sub cache invalidation
- ✅ Event audit trail with Redis Streams

## Performance
- **Single Service**: 90-98% API reduction (same as v0.3.0)
- **Multi-Service**: 95-99% API reduction (40-60% additional savings)
- **Latency**: <5ms p50, <20ms p99
- **Scalability**: 50K+ ops/sec per Redis node

## Breaking Changes
- None! DistributedCache has same API as EnhancedCache

## Migration Guide
\`\`\`python
# v0.3.0 (SQLite)
cache = EnhancedCache(db_path="cache.db")

# v0.4.0 (Redis) - same interface
cache = DistributedCache(backend='redis', redis_url='redis://localhost:6379')
\`\`\`
```

### Publish Steps
1. Merge to main: `git merge redis-backend`
2. Tag: `git tag v0.4.0`
3. Build: `maturin build --release`
4. Publish: `twine upload target/wheels/*.whl`
5. GitHub Release: Create release with wheel + notes

---

## Week 7: Launch & Feedback

### Rollout Plan
1. **Day 1**: Announce on GitHub, PyPI (v0.4.0 available)
2. **Day 2-3**: Outreach to top 10 users (ask for feedback)
3. **Week 2**: Blog post (architecture, benchmarks, use case)
4. **Week 3**: Community calls (demo, Q&A)
5. **Week 4**: Iterate based on feedback

### Success Metrics
- [ ] 50+ downloads in first week
- [ ] <2% error rate in production users
- [ ] Performance: <10ms p99 latency
- [ ] 5+ enterprise inquiries

---

## 🚀 Quick Checklist

### Before Starting
- [ ] Allocate 2 FTE engineers
- [ ] Set up Redis development environment
- [ ] Review Rust async patterns for Redis
- [ ] Schedule weekly sync meetings

### Week 1-2: Core
- [ ] Implement RedisCache struct
- [ ] Add redis dependency to Cargo.toml
- [ ] PyO3 bindings for DistributedCache
- [ ] Unit tests for get/put/deduplicate

### Week 3: Advanced
- [ ] Redis Cluster support
- [ ] Pub/Sub invalidation
- [ ] Redis Streams audit trail
- [ ] Integration tests

### Week 4: Testing
- [ ] Performance benchmarks
- [ ] Multi-service testing
- [ ] Failure scenario testing
- [ ] Memory profiling

### Week 5: Documentation
- [ ] README updates
- [ ] Docker Compose examples
- [ ] API reference
- [ ] Migration guide

### Week 6: Release
- [ ] Version bump to 0.4.0
- [ ] Release notes
- [ ] Wheel builds
- [ ] GitHub release

### Week 7: Launch
- [ ] Announce
- [ ] User outreach
- [ ] Blog post
- [ ] Community feedback

---

## Expected Outcomes

✅ **Shipped**: v0.4.0 with Redis backend
✅ **Market**: Enterprise-ready distributed caching
✅ **Savings**: 95-99% API cost reduction (40-60% additional)
✅ **Latency**: <5ms p50, sub-20ms p99
✅ **Scalability**: Multi-service, 50K+ ops/sec per node
✅ **Adoption**: 2x market expansion (multi-service deployments)

---

## Next Phase (After v0.4.0)

### v0.4.5 - Kafka/Streaming (4 weeks)
Real-time enrichment for live operations (delivery ETA, demand prediction)

### v0.5.0 - ML Prefetching (4 weeks)
Predictive cache warming, 30-50% additional savings

See **ROADMAP_v0.4_NEXT_STEPS.md** for full plan.
