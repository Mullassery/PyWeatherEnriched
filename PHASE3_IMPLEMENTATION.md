# Phase 3: Implementation Guide

**Duration**: Weeks 11-16 (6 weeks)  
**Modules to Add**: 4 new Rust modules + Python bindings  
**Code Size**: ~3,000 new lines of Rust code  
**Tests**: 50+ new test cases

---

## Module 1: Streaming (Base Infrastructure)

**File**: `src/streaming.rs` (✅ **Started**)  
**Status**: Foundation complete, ready for protocol implementations

### What's Implemented
```rust
✅ StreamingConfig - configuration management
✅ StreamingStats - metrics collection
✅ StreamingBatchProcessor - batch processing with buffer
✅ DeadLetterQueue - failed message handling
✅ RateLimiter - token bucket rate limiting
✅ EnrichmentResult - result model
```

### What's Needed

**Week 11.1: Kafka Adapter**
```rust
pub struct KafkaEnricher {
    consumer: rdkafka::consumer::StreamConsumer,
    producer: rdkafka::producer::FutureProducer,
    enricher: Arc<Enricher>,
    processor: StreamingBatchProcessor,
}

impl KafkaEnricher {
    pub async fn start_streaming(&self, input_topic: &str, output_topic: &str) -> Result<()>
    pub async fn consume_message(&self) -> Result<EnrichmentResult>
    pub async fn produce_result(&self, result: EnrichmentResult) -> Result<()>
}
```

**Checkpoint**: Kafka enrichment produces results to output topic with <100ms latency

**Week 11.2: MQTT Adapter**
```rust
pub struct MqttEnricher {
    client: rumqttc::AsyncClient,
    enricher: Arc<Enricher>,
}

impl MqttEnricher {
    pub async fn subscribe(&self, topic: &str) -> Result<()>
    pub async fn handle_message(&self, payload: &[u8]) -> Result<EnrichmentResult>
    pub async fn publish_result(&self, result: &EnrichedRow) -> Result<()>
}
```

**Checkpoint**: IoT devices can send sensor data → enriched with weather → publish results

**Week 11.3: HTTP Webhook Server**
```rust
pub async fn start_webhook_server(addr: &str, enricher: Arc<Enricher>) -> Result<()>

pub async fn enrich_webhook(
    State(enricher): State<Arc<Enricher>>,
    Json(rows): Json<Vec<Row>>,
) -> Json<Vec<EnrichedRow>>
```

**Checkpoint**: REST API accepts POST requests, enriches synchronously

---

## Module 2: Advanced Weather Data

**File**: `src/weather/advanced.rs` (New)  
**Status**: To be created

### Week 12.1: Air Quality Integration

```rust
pub struct AirQualityFetcher;

impl AirQualityFetcher {
    pub async fn fetch(location: &Location) -> Result<AirQuality>
    pub async fn fetch_with_components(location: &Location) -> Result<PollutionDetail>
}

#[derive(Serialize, Deserialize)]
pub struct AirQuality {
    pub aqi: u32,              // 1-5
    pub pm25: f32,             // µg/m³
    pub pm10: f32,
    pub no2: f32,
    pub o3: f32,
    pub so2: f32,
    pub co: f32,
}
```

**Data Sources**:
- OpenWeather Air Pollution API (free + paid)
- Alternative: Waqi API (World Air Quality Index)

**Checkpoint**: AQI data enriched for healthcare use cases

### Week 12.2: Disaster & Alerts

```rust
pub struct DisasterMonitor;

impl DisasterMonitor {
    pub async fn get_active_alerts(location: &Location) -> Result<Vec<DisasterAlert>>
    pub async fn check_heatwave_risk(location: &Location, temp: f32) -> Result<HeatwaveRisk>
    pub async fn check_flood_risk(location: &Location, rainfall_24h: f32) -> Result<FloodRisk>
}

pub enum DisasterType {
    Heatwave, Coldwave, Flood, Storm, Cyclone, Drought, Wildfire,
}

pub struct DisasterAlert {
    pub alert_type: DisasterType,
    pub severity: u8,
    pub description: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

**Data Sources**:
- OpenWeather alerts API
- NOAA (US)
- Local emergency management APIs

**Checkpoint**: Logistics platforms can re-route based on disaster alerts

### Week 12.3: Climate Context

```rust
pub struct ClimateContext {
    pub seasonal_phase: SeasonalPhase,
    pub deviation_from_normal: f32,
    pub monsoon_intensity: Option<f32>,
    pub el_nino_status: Option<String>,
    pub drought_risk: f32,
    pub flood_risk: f32,
}

impl ClimateContext {
    pub async fn fetch(location: &Location) -> Result<Self>
}
```

**Data Sources**:
- NOAA Climate Prediction Center
- India Meteorological Department
- Climate indices (El Niño, NAO, IOD)

**Checkpoint**: Agriculture & energy companies get seasonal context

### Week 12.4: Forecast Integration

```rust
pub struct WeatherForecast {
    pub forecast_date: DateTime<Utc>,
    pub forecast_hours: Vec<HourlyForecast>,
    pub confidence: f32,
}

impl ForecastFetcher {
    pub async fn fetch_forecast(
        location: &Location,
        hours_ahead: u32,  // 1-384 (16 days max)
    ) -> Result<WeatherForecast>
}
```

**Checkpoints by forecast range**:
- 1-3 hours: 90%+ accuracy
- 4-24 hours: 80-90% accuracy
- 1-5 days: 70-80% accuracy
- 5-14 days: 60-70% accuracy

---

## Module 3: Operational Features

**File**: `src/operations/mod.rs` (New)  
**Status**: To be created

### Week 13: Error Recovery & Backfill

```rust
pub struct EnrichmentRecovery {
    failed_records: Vec<FailedRecord>,
    retry_policy: RetryPolicy,
}

impl EnrichmentRecovery {
    pub async fn retry_failed(&mut self, enricher: &Enricher) -> Result<()>
    pub async fn export_to_dlq(&self, path: &str) -> Result<()>
    pub async fn resume_from_checkpoint(&self, checkpoint: &str) -> Result<()>
}
```

**Features**:
- Exponential backoff (2^n seconds)
- Configurable max retries (default 3)
- Dead-letter queue archival
- Checkpoint/resume capability

**Checkpoint**: Failed records can be reprocessed later

### Week 14: Multi-Tenancy

```rust
pub struct TenantManager {
    tenants: HashMap<String, TenantConfig>,
    rate_limiters: HashMap<String, RateLimiter>,
}

pub struct TenantConfig {
    pub tenant_id: String,
    pub api_key: String,
    pub rate_limit: RateLimit,
    pub cache_ttl: Duration,
}

impl TenantManager {
    pub async fn enrich_for_tenant(
        &self,
        tenant_id: &str,
        rows: Vec<Row>,
    ) -> Result<Vec<EnrichedRow>>
}
```

**Features**:
- Per-tenant API keys
- Rate limiting (requests/sec)
- Isolated caching
- Usage tracking

**Checkpoint**: SaaS platform can support multiple customers

### Week 15: Audit & Compliance

```rust
pub struct AuditLogger {
    storage: Box<dyn AuditStorage>,
}

pub struct AuditLog {
    pub timestamp: DateTime<Utc>,
    pub tenant_id: String,
    pub action: AuditAction,
    pub row_count: usize,
    pub cost_usd: f64,
    pub errors: Vec<String>,
}

impl AuditLogger {
    pub async fn log(&self, audit: AuditLog) -> Result<()>
}

pub trait AuditStorage: Send + Sync {
    async fn store(&self, log: AuditLog) -> Result<()>;
    async fn query(&self, filters: AuditFilter) -> Result<Vec<AuditLog>>;
}
```

**Compliance Standards**:
- GDPR: Data retention policies
- SOC2: Access logging
- HIPAA: Audit trails
- PCI-DSS: API key rotation

**Checkpoint**: Enterprise compliance requirements met

### Week 16: Testing & Release

- [ ] 50+ test cases for streaming
- [ ] Performance benchmarks (100K events/sec)
- [ ] Integration tests with Kafka/MQTT
- [ ] Documentation & deployment guides
- [ ] Beta release notes

---

## Python Bindings (Phase 3)

**File**: `src/python/streaming.py` (New)

```python
from pyweatherenriched import PyWeatherEnriched, StreamingConfig

# Kafka enrichment
enricher = PyWeatherEnriched(api_key="...")
streaming = enricher.create_kafka_stream(
    bootstrap_servers=["kafka:9092"],
    input_topic="operational_data",
    output_topic="enriched_data",
    config=StreamingConfig(batch_size=1000)
)
streaming.start()

# MQTT enrichment
mqtt_enricher = enricher.create_mqtt_stream(
    broker_address="mqtt.example.com",
    topic="sensors/+/data",
)
mqtt_enricher.start()

# Webhook server
server = enricher.create_webhook_server(
    port=8080,
    path="/enrich"
)
server.start()
```

---

## Testing Strategy

### Unit Tests (Week 15)
- [ ] Streaming buffer tests (5)
- [ ] Rate limiter tests (4)
- [ ] Dead-letter queue tests (3)
- [ ] AQI fetcher tests (5)
- [ ] Disaster alert tests (4)
- [ ] Climate context tests (3)
- [ ] Forecast tests (4)
- [ ] Tenant isolation tests (4)
- [ ] Audit logging tests (4)
- [ ] Total: 40+ tests

### Integration Tests (Week 16)
- [ ] Kafka end-to-end (3)
- [ ] MQTT end-to-end (3)
- [ ] HTTP webhook (3)
- [ ] Error recovery (2)
- [ ] Multi-tenant isolation (2)
- [ ] Total: 13 integration tests

### Performance Tests (Week 16)
- [ ] 100K events/sec throughput
- [ ] <100ms latency percentile
- [ ] Cache hit rate > 70%
- [ ] Memory usage < 500MB

---

## Dependencies to Add

```toml
# Phase 3 additions
rdkafka = "0.36"           # Kafka
rumqttc = "0.24"           # MQTT
axum = "0.7"               # HTTP server
tokio-stream = "0.1"       # Streaming utilities
prometheus = "0.13"        # Metrics
opentelemetry = "0.21"     # Tracing
tracing = "0.1"            # Logging
```

---

## Rollout Strategy

**Week 11**: Kafka + MQTT + Webhooks (beta)  
**Week 12**: Advanced weather (production)  
**Week 13**: Operational features (production)  
**Week 14-15**: Multi-tenancy + Audit (production)  
**Week 16**: Full release  

---

## Success Criteria

✅ 100K events/second throughput  
✅ <100ms end-to-end latency  
✅ 99.9% uptime SLA  
✅ Zero message loss (exactly-once)  
✅ GDPR/SOC2 compliance  
✅ 50+ test cases passing  
✅ Complete documentation  

---

**Next Phase**: Phase 4 - Geo-spatial & Enterprise (8 weeks)

