/// Abstract data source layer - handles loading from different backends
/// Local files, Redis, S3, GCS, HTTP, or hybrid

use anyhow::Result;
use std::sync::Arc;

pub use crate::geospatial::config::DataSource;

/// Trait for loading geospatial data from any source
pub trait GeoDataLoader: Send + Sync {
    /// Load raster data at specific tile ID
    /// tile_id format: "40_-74" (lat_lon)
    fn get_tile(&self, tile_id: &str) -> Result<Vec<u8>>;

    /// Load vector data at specific tile ID
    fn get_vector(&self, tile_id: &str) -> Result<String>;

    /// Preload frequently used tiles
    fn preload(&self, tile_ids: &[&str]) -> Result<()>;
}

/// Load from local files
pub struct LocalFileLoader {
    base_path: std::path::PathBuf,
    file_pattern: String,
}

impl LocalFileLoader {
    pub fn new(base_path: std::path::PathBuf, file_pattern: String) -> Self {
        LocalFileLoader {
            base_path,
            file_pattern,
        }
    }

    fn format_path(&self, tile_id: &str) -> std::path::PathBuf {
        let parts: Vec<&str> = tile_id.split('_').collect();
        let lat = parts.get(0).copied().unwrap_or("0");
        let lon = parts.get(1).copied().unwrap_or("0");

        let filename = self
            .file_pattern
            .replace("{lat}", lat)
            .replace("{lon}", lon);

        self.base_path.join(filename)
    }
}

impl GeoDataLoader for LocalFileLoader {
    fn get_tile(&self, tile_id: &str) -> Result<Vec<u8>> {
        let path = self.format_path(tile_id);
        std::fs::read(&path).map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path.display(), e))
    }

    fn get_vector(&self, tile_id: &str) -> Result<String> {
        let path = self.format_path(tile_id);
        std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path.display(), e))
    }

    fn preload(&self, tile_ids: &[&str]) -> Result<()> {
        // Preload tiles into memory or cache
        for tile_id in tile_ids {
            let _ = self.get_tile(tile_id);
        }
        Ok(())
    }
}

/// Load from Redis cache
pub struct RedisLoader {
    client: redis::Client,
    key_prefix: String,
}

impl RedisLoader {
    pub fn new(redis_url: &str, key_prefix: String) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(RedisLoader { client, key_prefix })
    }
}

impl GeoDataLoader for RedisLoader {
    fn get_tile(&self, tile_id: &str) -> Result<Vec<u8>> {
        let mut conn = self.client.get_connection()?;
        let key = format!("{}{}", self.key_prefix, tile_id);
        redis::cmd("GET")
            .arg(&key)
            .query::<Option<Vec<u8>>>(&mut conn)?
            .ok_or_else(|| anyhow::anyhow!("Tile {} not found in Redis", tile_id))
    }

    fn get_vector(&self, tile_id: &str) -> Result<String> {
        let mut conn = self.client.get_connection()?;
        let key = format!("{}{}", self.key_prefix, tile_id);
        redis::cmd("GET")
            .arg(&key)
            .query::<String>(&mut conn)
            .map_err(|e| anyhow::anyhow!("Failed to load from Redis: {}", e))
    }

    fn preload(&self, tile_ids: &[&str]) -> Result<()> {
        let mut conn = self.client.get_connection()?;
        for tile_id in tile_ids {
            let key = format!("{}{}", self.key_prefix, tile_id);
            let _: Option<Vec<u8>> = redis::cmd("GET").arg(&key).query(&mut conn)?;
        }
        Ok(())
    }
}

/// Load from HTTP endpoint with optional local cache
pub struct HttpLoader {
    base_url: String,
    file_pattern: String,
    local_cache_path: Option<std::path::PathBuf>,
    client: reqwest::blocking::Client,
}

impl HttpLoader {
    pub fn new(
        base_url: String,
        file_pattern: String,
        local_cache_path: Option<std::path::PathBuf>,
    ) -> Self {
        HttpLoader {
            base_url,
            file_pattern,
            local_cache_path,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn format_url(&self, tile_id: &str) -> String {
        let parts: Vec<&str> = tile_id.split('_').collect();
        let lat = parts.get(0).copied().unwrap_or("0");
        let lon = parts.get(1).copied().unwrap_or("0");

        let filename = self
            .file_pattern
            .replace("{lat}", lat)
            .replace("{lon}", lon);

        format!("{}{}", self.base_url, filename)
    }
}

impl GeoDataLoader for HttpLoader {
    fn get_tile(&self, tile_id: &str) -> Result<Vec<u8>> {
        // Try local cache first
        if let Some(cache_path) = &self.local_cache_path {
            let parts: Vec<&str> = tile_id.split('_').collect();
            let lat = parts.get(0).copied().unwrap_or("0");
            let lon = parts.get(1).copied().unwrap_or("0");
            let filename = self
                .file_pattern
                .replace("{lat}", lat)
                .replace("{lon}", lon);
            let cache_file = cache_path.join(&filename);

            if cache_file.exists() {
                return std::fs::read(&cache_file)
                    .map_err(|e| anyhow::anyhow!("Failed to read cache: {}", e));
            }
        }

        // Download from HTTP
        let url = self.format_url(tile_id);
        let response = self.client.get(&url).send()?;
        let data = response.bytes()?;

        // Save to local cache if configured
        if let Some(cache_path) = &self.local_cache_path {
            let parts: Vec<&str> = tile_id.split('_').collect();
            let lat = parts.get(0).copied().unwrap_or("0");
            let lon = parts.get(1).copied().unwrap_or("0");
            let filename = self
                .file_pattern
                .replace("{lat}", lat)
                .replace("{lon}", lon);
            let cache_file = cache_path.join(&filename);

            if let Some(parent) = cache_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let _ = std::fs::write(&cache_file, &data);
        }

        Ok(data.to_vec())
    }

    fn get_vector(&self, tile_id: &str) -> Result<String> {
        let bytes = self.get_tile(tile_id)?;
        String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    fn preload(&self, tile_ids: &[&str]) -> Result<()> {
        for tile_id in tile_ids {
            let _ = self.get_tile(tile_id);
        }
        Ok(())
    }
}

/// Hybrid loader - try multiple sources in order
pub struct HybridLoader {
    loaders: Vec<Arc<dyn GeoDataLoader>>,
}

impl HybridLoader {
    pub fn new(loaders: Vec<Arc<dyn GeoDataLoader>>) -> Self {
        HybridLoader { loaders }
    }
}

impl GeoDataLoader for HybridLoader {
    fn get_tile(&self, tile_id: &str) -> Result<Vec<u8>> {
        for loader in &self.loaders {
            if let Ok(data) = loader.get_tile(tile_id) {
                return Ok(data);
            }
        }
        Err(anyhow::anyhow!(
            "Tile {} not found in any source",
            tile_id
        ))
    }

    fn get_vector(&self, tile_id: &str) -> Result<String> {
        for loader in &self.loaders {
            if let Ok(data) = loader.get_vector(tile_id) {
                return Ok(data);
            }
        }
        Err(anyhow::anyhow!(
            "Tile {} not found in any source",
            tile_id
        ))
    }

    fn preload(&self, tile_ids: &[&str]) -> Result<()> {
        for loader in &self.loaders {
            let _ = loader.preload(tile_ids);
        }
        Ok(())
    }
}

/// Factory for creating loaders from config
pub fn create_loader(source: &DataSource) -> Result<Arc<dyn GeoDataLoader>> {
    match source {
        DataSource::LocalFile {
            base_path,
            file_pattern,
        } => {
            Ok(Arc::new(LocalFileLoader::new(
                base_path.clone(),
                file_pattern.clone(),
            )))
        }
        DataSource::Redis { url, key_prefix } => {
            Ok(Arc::new(RedisLoader::new(url, key_prefix.clone())?))
        }
        DataSource::HttpUrl {
            base_url,
            file_pattern,
            local_cache_path,
            ..
        } => Ok(Arc::new(HttpLoader::new(
            base_url.clone(),
            file_pattern.clone(),
            local_cache_path.clone(),
        ))),
        DataSource::Hybrid { sources } => {
            let mut loaders = Vec::new();
            for source in sources {
                loaders.push(create_loader(source)?);
            }
            Ok(Arc::new(HybridLoader::new(loaders)))
        }
        _ => Err(anyhow::anyhow!("S3/GCS loaders not yet implemented")),
    }
}
