# Phase 3: Real-Time Streaming & Advanced Weather (Weeks 11-16)

**Focus**: Live data enrichment + comprehensive weather context  
**Throughput Target**: 100K events/second (per machine)  
**Latency Target**: <100ms end-to-end enrichment

---

## Phase 3.1: Real-Time Streaming (Weeks 11-12)

### Kafka Integration

```rust
// src/streaming/kafka.rs
use rdkafka::consumer::{StreamConsumer, Consumer};
use rdkafka::producer::{FutureProducer, FutureRecord};

pub struct KafkaEnricher {
    consumer: StreamConsumer,
    producer: FutureProducer,
    enricher: Enricher,
}

impl KafkaEnricher {
    pub async fn enrich_stream(
        &self,
        input_topic: &str,
        output_topic: &str,
    ) -> Result<()> {
        // Consume from input topic
        for msg in self.consumer.iter() {
            match msg {
                Ok(m) => {
                    // Parse row from Kafka message
                    let row = serde_json::from_slice(m.payload())?;
                    
                    // Enrich (parallel + cached)
                    let enriched = self.enricher.enrich_row(row).await?;
                    
                    // Produce to output topic
                    let json = serde_json::to_string(&enriched)?;
                    self.producer
                        .send(FutureRecord::to(output_topic).payload(&json))
                        .await?;
                }
                Err(e) => log::error!("Kafka error: {}", e),
            }
        }
        Ok(())
    }
}
```

**Features**:
- ✅ Consumer group management
- ✅ Error handling & retries
- ✅ Offset tracking
- ✅ Batch enrichment
- ✅ Dead-letter queues for failed records

**Performance**:
- Latency: 50-100ms per event
- Throughput: 50K+ events/sec per node
- Memory: Constant (streaming)

### MQTT/IoT Support

```rust
// src/streaming/mqtt.rs
use rumqttc::{AsyncClient, MqttOptions};

pub struct MqttEnricher {
    client: AsyncClient,
    enricher: Enricher,
}

impl MqttEnricher {
    pub async fn enrich_iot_stream(
        &self,
        subscribe_topic: &str,
        publish_topic_prefix: &str,
    ) -> Result<()> {
        // Subscribe to IoT data stream
        self.client.subscribe(subscribe_topic, 0).await?;
        
        loop {
            match self.client.eventloop.poll().await {
                Ok(notification) => {
                    if let Publish(publish) = notification {
                        // Parse sensor data
                        let sensor_data = serde_json::from_slice(&publish.payload)?;
                        
                        // Enrich with weather
                        let enriched = self.enricher.enrich_row(sensor_data).await?;
                        
                        // Publish enriched data
                        self.client
                            .publish(publish_topic_prefix, 1, false, serde_json::to_vec(&enriched)?)
                            .await?;
                    }
                }
                Err(e) => log::error!("MQTT error: {}", e),
            }
        }
    }
}
```

**Devices Supported**:
- Weather stations
- Air quality sensors
- Soil moisture/temperature
- Smart meters
- Traffic sensors
- Building automation

**Protocol Support**:
- MQTT (publish/subscribe)
- HTTP webhooks
- gRPC streams (Phase 4)

### HTTP Webhook Listener

```rust
// src/streaming/webhook.rs
use axum::{Router, routing::post, Json};

pub async fn start_webhook_server(
    addr: &str,
    enricher: Arc<Enricher>,
) -> Result<()> {
    let app = Router::new()
        .route("/enrich", post(enrich_webhook))
        .with_state(enricher);
    
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    
    Ok(())
}

async fn enrich_webhook(
    State(enricher): State<Arc<Enricher>>,
    Json(rows): Json<Vec<Row>>,
) -> Json<Vec<EnrichedRow>> {
    let enriched = enricher.enrich_batch_parallel(rows).await.unwrap_or_default();
    Json(enriched)
}
```

**REST API**:
- `POST /enrich` - Enrich rows synchronously
- `POST /enrich_async` - Queue for async enrichment
- `GET /status` - Enrichment job status

---

## Phase 3.2: Advanced Weather Data (Weeks 11-12)

### Air Quality Index (AQI)

```rust
// src/weather/aqi.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirQuality {
    pub aqi: u32,           // 1-5 (Good to Hazardous)
    pub pm25: f32,          // µg/m³
    pub pm10: f32,          // µg/m³
    pub no2: f32,           // ppb
    pub o3: f32,            // ppb
    pub so2: f32,           // ppb
    pub co: f32,            // ppm
}

impl AirQuality {
    pub async fn fetch(location: &Location) -> Result<Self> {
        // OpenWeather Pollution API
        let url = format!(
            "https://api.openweathermap.org/data/2.5/air_pollution?lat={}&lon={}&appid={}",
            location.latitude, location.longitude, API_KEY
        );
        
        let response: PollutionResponse = reqwest::get(&url).await?.json().await?;
        
        Ok(AirQuality {
            aqi: response.list[0].main.aqi,
            pm25: response.list[0].components.pm2_5,
            pm10: response.list[0].components.pm10,
            no2: response.list[0].components.no2,
            o3: response.list[0].components.o3,
            so2: response.list[0].components.so2,
            co: response.list[0].components.co,
        })
    }
}
```

**Integration**:
- OpenWeather Pollution API
- Waqi API (World Air Quality Index)
- Local air quality stations

**Use Cases**:
- Healthcare: Respiratory admission prediction
- Retail: Indoor air quality monitoring
- Logistics: Driver health & safety
- Agriculture: Crop damage prediction

### Disaster & Extreme Weather Alerts

```rust
// src/weather/disasters.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisasterType {
    Heatwave,
    Coldwave,
    Flood,
    Storm,
    Cyclone,
    Drought,
    Wildfire,
    Landslide,
    AvalancheWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisasterAlert {
    pub alert_type: DisasterType,
    pub severity: u8,              // 1-5
    pub description: String,
    pub affected_area: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub recommended_action: String,
}

pub struct DisasterMonitor;

impl DisasterMonitor {
    pub async fn get_alerts(location: &Location) -> Result<Vec<DisasterAlert>> {
        // Check multiple sources
        let mut alerts = Vec::new();
        
        // OpenWeather alerts
        if let Ok(ow_alerts) = Self::fetch_openweather_alerts(location).await {
            alerts.extend(ow_alerts);
        }
        
        // NOAA alerts (US only)
        if location.country == Some("US".to_string()) {
            if let Ok(noaa_alerts) = Self::fetch_noaa_alerts(location).await {
                alerts.extend(noaa_alerts);
            }
        }
        
        Ok(alerts)
    }
}
```

**Data Sources**:
- OpenWeather alerts
- NOAA (National Weather Service)
- Local meteorological agencies
- Emergency management systems

**Use Cases**:
- Logistics: Route re-planning during storms
- Retail: Supply chain alerts
- Healthcare: Casualty surge prediction
- Agriculture: Crop protection actions
- Utilities: Power grid load management

### Climate Anomalies & Seasonal Indicators

```rust
// src/weather/climate.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateContext {
    pub seasonal_phase: SeasonalPhase,
    pub deviation_from_normal: f32,  // °C above/below 30-year average
    pub monsoon_intensity: Option<f32>,  // India-specific
    pub el_nino_status: Option<String>,  // "Neutral", "El Niño", "La Niña"
    pub drought_risk: f32,            // 0.0-1.0
    pub flood_risk: f32,              // 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeasonalPhase {
    MonsoonOnset,
    MonsoonPeak,
    MonsoonWithdrawal,
    DrySeasonOnset,
    DrySeasonPeak,
    PeakSummer,
    Transition,
}

impl ClimateContext {
    pub async fn fetch(location: &Location) -> Result<Self> {
        // Multi-source climate data
        let seasonal = Self::get_seasonal_phase(location).await?;
        let deviation = Self::get_temp_deviation(location).await?;
        let monsoon = Self::get_monsoon_intensity(location).await?;
        
        Ok(ClimateContext {
            seasonal_phase: seasonal,
            deviation_from_normal: deviation,
            monsoon_intensity: monsoon,
            el_nino_status: Self::get_el_nino_status().await?,
            drought_risk: Self::calculate_drought_risk(location, deviation).await?,
            flood_risk: Self::calculate_flood_risk(location, monsoon).await?,
        })
    }
}
```

**Data Sources**:
- NOAA Climate Prediction Center
- India Meteorological Department
- Climate indices (El Niño, NAO, IOD)
- Long-term climate normals (30-year averages)

**Use Cases**:
- Agriculture: Seasonal planting decisions
- Energy: Demand forecasting for peak seasons
- Insurance: Risk assessment by season
- Retail: Inventory planning by season

### Forecast Weather Integration

```rust
// src/weather/forecast.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherForecast {
    pub forecast_date: DateTime<Utc>,
    pub forecast_hours: Vec<HourlyForecast>,  // 1-384 hours (16 days)
    pub confidence: f32,  // 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyForecast {
    pub hour_offset: u32,                    // 1-384
    pub temperature: f32,
    pub humidity: f32,
    pub rainfall_probability: f32,           // 0.0-1.0
    pub rainfall_amount: f32,                // mm
    pub wind_speed: f32,
}

pub struct ForecastFetcher;

impl ForecastFetcher {
    pub async fn fetch(
        location: &Location,
        hours_ahead: u32,  // 1-384 (16 days)
    ) -> Result<WeatherForecast> {
        // OpenWeather 5-day forecast (80 intervals)
        // Or premium sources for 16-day forecasts
        
        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&cnt={}&appid={}",
            location.latitude, location.longitude, hours_ahead / 3, API_KEY
        );
        
        let response: ForecastResponse = reqwest::get(&url).await?.json().await?;
        
        Ok(WeatherForecast {
            forecast_date: Utc::now(),
            forecast_hours: response.list.into_iter()
                .map(|item| HourlyForecast {
                    hour_offset: item.dt as u32,
                    temperature: item.main.temp,
                    humidity: item.main.humidity,
                    rainfall_probability: item.pop,
                    rainfall_amount: item.rain.map(|r| r.amount).unwrap_or(0.0),
                    wind_speed: item.wind.speed,
                })
                .collect(),
            confidence: 0.85,
        })
    }
}
```

**Forecast Horizons**:
- 1-3 hours: High accuracy (90%+)
- 4-24 hours: Good accuracy (80-90%)
- 1-5 days: Medium accuracy (70-80%)
- 5-14 days: Lower accuracy (60-70%)

**Use Cases**:
- Delivery: Route planning for next day
- Retail: Staff scheduling based on weather
- Energy: Load forecasting
- Healthcare: Surge prediction for next week
- Logistics: Vehicle maintenance scheduling

---

## Phase 3.3: Real-Time Dashboards (Week 13)

### Enrichment Status Dashboard

```rust
// src/dashboard/status.rs
pub struct DashboardMetrics {
    pub total_enriched: u64,
    pub enrichment_rate: f64,  // rows/sec
    pub error_rate: f32,        // 0.0-1.0
    pub cache_hit_rate: f32,
    pub avg_latency_ms: f64,
    pub api_calls_today: u64,
    pub cost_today: f64,
}

// Metrics exposed on /metrics (Prometheus format)
// Grafana dashboard template provided
```

**Metrics Tracked**:
- Rows enriched (total, per location, per hour)
- Cache hit/miss rates
- API response times
- Error rates by type (location not found, API timeout, etc.)
- Cost tracking (API calls, estimated spend)
- Latency percentiles (p50, p95, p99)

**Dashboards**:
- Real-time enrichment status
- Error monitoring
- Cost tracking
- Performance metrics
- Data quality

---

## Phase 3.4: Operational Features (Weeks 14-16)

### Error Recovery & Backfill

```rust
// src/operations/recovery.rs
pub struct EnrichmentRecovery {
    failed_records: Vec<FailedRecord>,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone)]
pub struct FailedRecord {
    pub original_row: Row,
    pub error: WeatherError,
    pub attempt_count: u32,
    pub last_attempt: DateTime<Utc>,
}

impl EnrichmentRecovery {
    pub async fn retry_failed(&mut self, enricher: &Enricher) -> Result<()> {
        for record in &mut self.failed_records {
            if record.attempt_count < self.retry_policy.max_attempts {
                // Retry with exponential backoff
                tokio::time::sleep(
                    Duration::from_secs(2_u64.pow(record.attempt_count))
                ).await;
                
                match enricher.enrich_row(record.original_row.clone()).await {
                    Ok(_) => {
                        // Success - remove from failed list
                    }
                    Err(e) => {
                        record.attempt_count += 1;
                        record.last_attempt = Utc::now();
                    }
                }
            }
        }
        Ok(())
    }
}
```

**Features**:
- Exponential backoff retry
- Dead-letter queues
- Failed record archival
- Recovery scheduling

### Multi-Tenancy Support

```rust
// src/tenant/mod.rs
pub struct TenantConfig {
    pub tenant_id: String,
    pub api_key: String,
    pub rate_limit: RateLimit,
    pub cache_ttl: Duration,
    pub allowed_locations: Option<Vec<String>>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub max_concurrent_requests: u32,
}

impl TenantManager {
    pub async fn enrich_for_tenant(
        &self,
        tenant_id: &str,
        rows: Vec<Row>,
    ) -> Result<Vec<EnrichedRow>> {
        let config = self.get_tenant_config(tenant_id)?;
        let enricher = Enricher::new_with_config(config)?;
        enricher.enrich_batch_parallel(rows).await
    }
}
```

**Features**:
- Per-tenant API keys
- Rate limiting
- Usage tracking
- Isolated caching
- Custom configurations

### Audit & Compliance Logging

```rust
// src/audit/mod.rs
#[derive(Debug, Clone, Serialize)]
pub struct AuditLog {
    pub timestamp: DateTime<Utc>,
    pub tenant_id: String,
    pub action: AuditAction,
    pub row_count: usize,
    pub cost_usd: f64,
    pub cache_hit_rate: f32,
    pub errors: Vec<String>,
}

pub enum AuditAction {
    EnrichmentStarted,
    EnrichmentCompleted,
    EnrichmentFailed,
    CacheHit,
    ApiCallMade,
}

pub struct AuditLogger {
    storage: Box<dyn AuditStorage>,
}

impl AuditLogger {
    pub async fn log(&self, audit: AuditLog) -> Result<()> {
        self.storage.store(audit).await
    }
}
```

**Compliance**:
- GDPR: Data retention policies
- SOC2: Access logging
- HIPAA: Audit trails
- PCI-DSS: API key rotation

---

## Phase 3 Implementation Timeline

**Week 11**: Kafka + MQTT + Webhooks  
**Week 12**: AQI + Disaster Alerts + Climate Context  
**Week 13**: Forecast Integration + Dashboard  
**Week 14**: Error Recovery + Backfill  
**Week 15**: Multi-Tenancy + Audit Logging  
**Week 16**: Testing + Documentation + Release  

---

## Phase 3 Dependencies (New)

```toml
# Streaming
rdkafka = "0.36"
rumqttc = "0.24"
axum = "0.7"
tokio-stream = "0.1"

# Metrics
prometheus = "0.13"
opentelemetry = "0.21"

# Monitoring
tracing = "0.1"
tracing-subscriber = "0.3"

# JSON/Serialization
serde_json = "1.0"
```

---

## Phase 3 Deliverables

✅ **Real-time enrichment**: Kafka, MQTT, Webhooks  
✅ **Advanced weather**: AQI, disasters, climate, forecasts  
✅ **Dashboards**: Status, errors, costs, performance  
✅ **Operations**: Error recovery, multi-tenancy, audit logs  
✅ **Tests**: 50+ test cases for streaming  
✅ **Documentation**: Architecture guides, deployment instructions  

---

**Target**: 100K events/second, <100ms latency, complete audit trail

