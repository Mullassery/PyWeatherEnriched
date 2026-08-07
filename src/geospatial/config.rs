/// Geospatial configuration - user controls where data comes from
///
/// Users can configure:
/// - Local file cache (fastest, requires manual download)
/// - Redis cache (shared across services)
/// - HTTP download (on-demand from public APIs)
/// - S3/GCS (cloud storage)
/// - Hybrid (try each in order)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeospatialConfig {
    /// Critical layers configuration
    pub elevation: DataSourceConfig,
    pub urban_heat_island: DataSourceConfig,

    /// Optional layers configuration
    pub vegetation: Option<DataSourceConfig>,
    pub soil: Option<DataSourceConfig>,
    pub flood_risk: Option<DataSourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub source: DataSource,
    pub cache_ttl_seconds: u32,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DataSource {
    /// Load from local files
    /// Example: /data/srtm/srtm30m_n40_e-074_COG.tif
    LocalFile {
        base_path: PathBuf,
        file_pattern: String, // "{lat}_{lon}.tif"
    },

    /// Load from Redis (for shared caching)
    /// Example: redis://localhost:6379
    Redis {
        url: String,
        key_prefix: String, // "geo:elevation:"
    },

    /// Download on-demand from HTTP endpoint
    /// Example: https://lpdaac.usgs.gov/appeears/api/.../
    HttpUrl {
        base_url: String,
        file_pattern: String, // "{lat}_{lon}.tif"
        cache_locally: bool,
        local_cache_path: Option<PathBuf>,
    },

    /// Load from S3
    /// Example: s3://bucket/path/
    S3 {
        bucket: String,
        region: String,
        prefix: String,
        file_pattern: String,
    },

    /// Load from Google Cloud Storage
    /// Example: gs://bucket/path/
    GCS {
        bucket: String,
        prefix: String,
        file_pattern: String,
    },

    /// Try multiple sources in order
    /// Falls back to next if current fails
    Hybrid {
        sources: Vec<Box<DataSource>>,
    },
}

impl GeospatialConfig {
    /// Create config with all defaults
    pub fn default_local(base_path: PathBuf) -> Self {
        GeospatialConfig {
            elevation: DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path: base_path.clone(),
                    file_pattern: "srtm/{lat}_{lon}_COG.tif".to_string(),
                },
                cache_ttl_seconds: 86400 * 30, // 30 days
                timeout_seconds: 10,
            },
            urban_heat_island: DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path: base_path.clone(),
                    file_pattern: "osm/{lat}_{lon}.geojson".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 10,
            },
            vegetation: Some(DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path: base_path.clone(),
                    file_pattern: "sentinel/ndvi/{year}_{month:02d}.tif".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 10,
            }),
            soil: Some(DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path: base_path.clone(),
                    file_pattern: "soilgrids/soil_{lat}_{lon}.tif".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 10,
            }),
            flood_risk: Some(DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path,
                    file_pattern: "hazards/flood_risk_{lat}_{lon}.tif".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 10,
            }),
        }
    }

    /// Create config with Redis caching
    pub fn with_redis(redis_url: &str) -> Self {
        GeospatialConfig {
            elevation: DataSourceConfig {
                source: DataSource::Redis {
                    url: redis_url.to_string(),
                    key_prefix: "geo:elevation:".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 5,
            },
            urban_heat_island: DataSourceConfig {
                source: DataSource::Redis {
                    url: redis_url.to_string(),
                    key_prefix: "geo:uhi:".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 5,
            },
            vegetation: Some(DataSourceConfig {
                source: DataSource::Redis {
                    url: redis_url.to_string(),
                    key_prefix: "geo:ndvi:".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 5,
            }),
            soil: Some(DataSourceConfig {
                source: DataSource::Redis {
                    url: redis_url.to_string(),
                    key_prefix: "geo:soil:".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 5,
            }),
            flood_risk: Some(DataSourceConfig {
                source: DataSource::Redis {
                    url: redis_url.to_string(),
                    key_prefix: "geo:flood:".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 5,
            }),
        }
    }

    /// Create config with S3 storage
    pub fn with_s3(bucket: &str, region: &str) -> Self {
        GeospatialConfig {
            elevation: DataSourceConfig {
                source: DataSource::S3 {
                    bucket: bucket.to_string(),
                    region: region.to_string(),
                    prefix: "srtm/".to_string(),
                    file_pattern: "{lat}_{lon}_COG.tif".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 15,
            },
            urban_heat_island: DataSourceConfig {
                source: DataSource::S3 {
                    bucket: bucket.to_string(),
                    region: region.to_string(),
                    prefix: "osm/".to_string(),
                    file_pattern: "{lat}_{lon}.geojson".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 15,
            },
            vegetation: None,  // Optional: user must enable if needed
            soil: None,
            flood_risk: None,
        }
    }

    /// Create hybrid config - try local first, fall back to HTTP
    pub fn hybrid(local_path: PathBuf, http_base: &str) -> Self {
        GeospatialConfig {
            elevation: DataSourceConfig {
                source: DataSource::Hybrid {
                    sources: vec![
                        Box::new(DataSource::LocalFile {
                            base_path: local_path.clone(),
                            file_pattern: "srtm/{lat}_{lon}_COG.tif".to_string(),
                        }),
                        Box::new(DataSource::HttpUrl {
                            base_url: http_base.to_string(),
                            file_pattern: "srtm/{lat}_{lon}.tif".to_string(),
                            cache_locally: true,
                            local_cache_path: Some(local_path.clone()),
                        }),
                    ],
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 30,
            },
            urban_heat_island: DataSourceConfig {
                source: DataSource::LocalFile {
                    base_path: local_path.clone(),
                    file_pattern: "osm/{lat}_{lon}.geojson".to_string(),
                },
                cache_ttl_seconds: 86400 * 30,
                timeout_seconds: 10,
            },
            vegetation: None,
            soil: None,
            flood_risk: None,
        }
    }

    /// From TOML/JSON config file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: GeospatialConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}

/* Example config file (geospatial.toml)
 *
 *
 * [elevation]
 * source = "local_file"
 * base_path = "/data/geospatial"
 * file_pattern = "srtm/{lat}_{lon}_COG.tif"
 * cache_ttl_seconds = 2592000
 * timeout_seconds = 10
 *
 * [urban_heat_island]
 * source = "redis"
 * url = "redis://localhost:6379"
 * key_prefix = "geo:uhi:"
 * cache_ttl_seconds = 2592000
 * timeout_seconds = 5
 *
 * [vegetation]
 * source = "s3"
 * bucket = "my-geospatial-data"
 * region = "us-west-2"
 * prefix = "sentinel/"
 * file_pattern = "ndvi/{year}_{month:02d}.tif"
 * cache_ttl_seconds = 2592000
 * timeout_seconds = 15
 *
 * [soil]
 * # Not configured - optional layer disabled
 * enabled = false
 *
 * [flood_risk]
 * # Not configured - optional layer disabled
 * enabled = false
 */
