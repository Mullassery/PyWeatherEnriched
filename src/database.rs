use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub connection_string: String,
    pub pool_size: usize,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    Snowflake,
    BigQuery,
    Redshift,
    MongoDB,
    DynamoDB,
}

#[derive(Clone, Debug)]
pub struct EnrichedRecord {
    pub id: String,
    pub location: String,
    pub timestamp: String,
    pub weather_data: HashMap<String, String>,
    pub enrichment_timestamp: u64,
}

#[async_trait]
pub trait DatabaseWriter: Send + Sync {
    async fn connect(&mut self) -> Result<(), String>;
    async fn write_record(&mut self, record: EnrichedRecord) -> Result<(), String>;
    async fn write_batch(&mut self, records: Vec<EnrichedRecord>) -> Result<usize, String>;
    async fn create_table_if_not_exists(&mut self) -> Result<(), String>;
    async fn close(&mut self) -> Result<(), String>;
    async fn get_connection_stats(&self) -> Result<ConnectionStats, String>;
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_records_written: u64,
    pub successful_writes: u64,
    pub failed_writes: u64,
    pub avg_latency_ms: f64,
    pub connection_uptime_seconds: u64,
    pub last_error: Option<String>,
}

pub struct PostgreSQLWriter {
    config: DatabaseConfig,
    stats: ConnectionStats,
}

impl PostgreSQLWriter {
    pub fn new(config: DatabaseConfig) -> Self {
        PostgreSQLWriter {
            config,
            stats: ConnectionStats {
                total_records_written: 0,
                successful_writes: 0,
                failed_writes: 0,
                avg_latency_ms: 0.0,
                connection_uptime_seconds: 0,
                last_error: None,
            },
        }
    }
}

#[async_trait]
impl DatabaseWriter for PostgreSQLWriter {
    async fn connect(&mut self) -> Result<(), String> {
        // Implementation would use sqlx to connect
        Ok(())
    }

    async fn write_record(&mut self, record: EnrichedRecord) -> Result<(), String> {
        self.stats.total_records_written += 1;
        self.stats.successful_writes += 1;
        Ok(())
    }

    async fn write_batch(&mut self, records: Vec<EnrichedRecord>) -> Result<usize, String> {
        let count = records.len();
        self.stats.total_records_written += count as u64;
        self.stats.successful_writes += count as u64;
        Ok(count)
    }

    async fn create_table_if_not_exists(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn get_connection_stats(&self) -> Result<ConnectionStats, String> {
        Ok(self.stats.clone())
    }
}

pub struct DatabasePool {
    writers: Vec<Box<dyn DatabaseWriter>>,
    current_index: usize,
}

impl DatabasePool {
    pub fn new() -> Self {
        DatabasePool {
            writers: Vec::new(),
            current_index: 0,
        }
    }

    pub async fn add_writer(&mut self, mut writer: Box<dyn DatabaseWriter>) -> Result<(), String> {
        writer.connect().await?;
        writer.create_table_if_not_exists().await?;
        self.writers.push(writer);
        Ok(())
    }

    pub async fn write_batch(&mut self, records: Vec<EnrichedRecord>) -> Result<usize, String> {
        if self.writers.is_empty() {
            return Err("No writers configured".to_string());
        }

        let writer = &mut self.writers[self.current_index];
        let result = writer.write_batch(records).await?;

        self.current_index = (self.current_index + 1) % self.writers.len();
        Ok(result)
    }

    pub async fn close_all(&mut self) -> Result<(), String> {
        for writer in &mut self.writers {
            writer.close().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SnowflakeConfig {
    pub account: String,
    pub warehouse: String,
    pub database: String,
    pub schema: String,
    pub table: String,
}

pub struct SnowflakeWriter {
    config: DatabaseConfig,
    snowflake_config: SnowflakeConfig,
    stats: ConnectionStats,
}

impl SnowflakeWriter {
    pub fn new(config: DatabaseConfig, snowflake_config: SnowflakeConfig) -> Self {
        SnowflakeWriter {
            config,
            snowflake_config,
            stats: ConnectionStats {
                total_records_written: 0,
                successful_writes: 0,
                failed_writes: 0,
                avg_latency_ms: 0.0,
                connection_uptime_seconds: 0,
                last_error: None,
            },
        }
    }
}

#[async_trait]
impl DatabaseWriter for SnowflakeWriter {
    async fn connect(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn write_record(&mut self, record: EnrichedRecord) -> Result<(), String> {
        self.stats.total_records_written += 1;
        self.stats.successful_writes += 1;
        Ok(())
    }

    async fn write_batch(&mut self, records: Vec<EnrichedRecord>) -> Result<usize, String> {
        let count = records.len();
        self.stats.total_records_written += count as u64;
        self.stats.successful_writes += count as u64;
        Ok(count)
    }

    async fn create_table_if_not_exists(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn get_connection_stats(&self) -> Result<ConnectionStats, String> {
        Ok(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_new() {
        let config = DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            connection_string: "postgresql://localhost".to_string(),
            pool_size: 10,
            timeout_seconds: 30,
            max_retries: 3,
        };

        assert_eq!(config.db_type, DatabaseType::PostgreSQL);
        assert_eq!(config.pool_size, 10);
    }

    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats {
            total_records_written: 1000,
            successful_writes: 950,
            failed_writes: 50,
            avg_latency_ms: 5.5,
            connection_uptime_seconds: 3600,
            last_error: Some("timeout".to_string()),
        };

        assert_eq!(stats.total_records_written, 1000);
        assert_eq!(stats.failed_writes, 50);
    }

    #[test]
    fn test_enriched_record() {
        let record = EnrichedRecord {
            id: "123".to_string(),
            location: "NYC".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            weather_data: HashMap::new(),
            enrichment_timestamp: 1704067200,
        };

        assert_eq!(record.id, "123");
    }

    #[tokio::test]
    async fn test_postgresql_writer() {
        let config = DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            connection_string: "postgresql://localhost".to_string(),
            pool_size: 10,
            timeout_seconds: 30,
            max_retries: 3,
        };

        let mut writer = PostgreSQLWriter::new(config);
        assert!(writer.connect().await.is_ok());
    }
}
