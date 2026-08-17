use crate::types::EnrichedData;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use lru::LruCache;
use rusqlite::{params, Connection, OptionalExtension};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

/// Enhanced cache key combining location, timestamp, and geohash for proximity matching
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CacheKey {
    geohash: String,
    timestamp: String,
}

/// Cache entry with TTL and metadata
#[derive(Clone, Debug)]
struct CacheEntry {
    data: EnrichedData,
    created_at: DateTime<Utc>,
    ttl_hours: i64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        let expires_at = self.created_at + Duration::hours(self.ttl_hours);
        Utc::now() > expires_at
    }
}

/// Geospatial proximity matcher for nearby location clustering
#[derive(Clone, Debug)]
pub struct LocationProximity {
    pub latitude: f64,
    pub longitude: f64,
    pub radius_km: f64,
}

impl LocationProximity {
    pub fn new(latitude: f64, longitude: f64, radius_km: f64) -> Self {
        LocationProximity {
            latitude,
            longitude,
            radius_km,
        }
    }

    /// Haversine distance between two coordinates in km
    fn distance_to(&self, other_lat: f64, other_lon: f64) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;
        let lat1_rad = self.latitude.to_radians();
        let lat2_rad = other_lat.to_radians();
        let delta_lat = (other_lat - self.latitude).to_radians();
        let delta_lon = (other_lon - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        EARTH_RADIUS_KM * c
    }

    fn is_nearby(&self, other_lat: f64, other_lon: f64) -> bool {
        self.distance_to(other_lat, other_lon) <= self.radius_km
    }
}

/// Date range for temporal caching
#[derive(Clone, Debug)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        DateRange { start, end }
    }

    /// Whether `dt` falls within this range (inclusive).
    pub fn contains(&self, dt: &DateTime<Utc>) -> bool {
        dt >= &self.start && dt <= &self.end
    }

    /// Whether this range shares any time with `other`.
    pub fn overlaps(&self, other: &DateRange) -> bool {
        self.start <= other.end && self.end >= other.start
    }
}

/// Statistics for cache performance
#[derive(Clone, Debug)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub proximity_hits: usize,
    pub range_hits: usize,
    pub deduplication_saves: usize,
    pub size: usize,
}

/// Enhanced weather cache with temporal and geospatial optimization
pub struct EnhancedCache {
    // Memory tier: LRU cache for frequently accessed entries
    memory: Mutex<LruCache<CacheKey, CacheEntry>>,
    // Persistent tier: SQLite for cross-session caching
    db: Option<Arc<Mutex<Connection>>>,
    // Statistics
    stats: Mutex<CacheStats>,
    // Configuration
    ttl_hours: i64,
    proximity_radius_km: f64,
    enable_persistence: bool,
}

impl EnhancedCache {
    /// Create a new enhanced cache with default settings
    pub fn new(cache_size: usize) -> Result<Self> {
        let size = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        Ok(EnhancedCache {
            memory: Mutex::new(LruCache::new(size)),
            db: None,
            stats: Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                proximity_hits: 0,
                range_hits: 0,
                deduplication_saves: 0,
                size: 0,
            }),
            ttl_hours: 24,
            proximity_radius_km: 5.0,
            enable_persistence: false,
        })
    }

    /// Create enhanced cache with SQLite persistence
    pub fn with_persistence(cache_size: usize, db_path: &str) -> Result<Self> {
        let size = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        let db = Connection::open(db_path)?;

        // Initialize schema
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS weather_cache (
                geohash TEXT NOT NULL,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                timestamp TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL NOT NULL,
                condition TEXT NOT NULL,
                location TEXT NOT NULL,
                created_at TEXT NOT NULL,
                ttl_hours INTEGER NOT NULL,
                PRIMARY KEY (geohash, timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_geohash_created
                ON weather_cache(geohash, created_at);
            CREATE INDEX IF NOT EXISTS idx_timestamp
                ON weather_cache(timestamp);",
        )?;

        Ok(EnhancedCache {
            memory: Mutex::new(LruCache::new(size)),
            db: Some(Arc::new(Mutex::new(db))),
            stats: Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                proximity_hits: 0,
                range_hits: 0,
                deduplication_saves: 0,
                size: 0,
            }),
            ttl_hours: 24,
            proximity_radius_km: 5.0,
            enable_persistence: true,
        })
    }

    /// Set proximity radius for geospatial clustering (in km)
    pub fn set_proximity_radius(&mut self, radius_km: f64) {
        self.proximity_radius_km = radius_km;
    }

    /// Set TTL for cache entries (in hours)
    pub fn set_ttl(&mut self, hours: i64) {
        self.ttl_hours = hours;
    }

    /// Get weather data with intelligent multi-tier lookup.
    ///
    /// `location` isn't part of the lookup key (entries are keyed by a
    /// geohash derived from `latitude`/`longitude` plus `timestamp` — see
    /// `put`), but is kept as a parameter for symmetry with `put` and
    /// because callers naturally have it on hand.
    pub fn get(
        &self,
        _location: &str,
        latitude: f64,
        longitude: f64,
        timestamp: &str,
    ) -> Option<EnrichedData> {
        // Create geohash-based key for proximity matching (quantize to ~5km grid)
        let geohash = format!(
            "{}_{}",
            (latitude * 100.0).round() as i32,
            (longitude * 100.0).round() as i32
        );
        let key = CacheKey {
            geohash,
            timestamp: timestamp.to_string(),
        };

        // Check memory tier first
        if let Ok(mut mem) = self.memory.lock() {
            if let Some(entry) = mem.get(&key) {
                if !entry.is_expired() {
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.hits += 1;
                    }
                    return Some(entry.data.clone());
                }
            }
        }

        // Check persistent tier
        if self.enable_persistence {
            if let Some(db_arc) = &self.db {
                if let Ok(db) = db_arc.lock() {
                    if let Ok(Some(data)) = self.get_from_db(&db, &key) {
                        if !data.1.is_expired() {
                            if let Ok(mut mem) = self.memory.lock() {
                                mem.put(key, data.1.clone());
                            }
                            if let Ok(mut stats) = self.stats.lock() {
                                stats.hits += 1;
                            }
                            return Some(data.0);
                        }
                    }
                }
            }
        }

        // Try proximity-based lookup (nearby locations)
        if let Some(nearby) = self.find_nearby_cached(latitude, longitude, timestamp) {
            if let Ok(mut stats) = self.stats.lock() {
                stats.proximity_hits += 1;
            }
            return Some(nearby);
        }

        if let Ok(mut stats) = self.stats.lock() {
            stats.misses += 1;
        }
        None
    }

    /// Cache weather data with TTL
    pub fn put(&self, data: EnrichedData) -> Result<()> {
        let geohash = format!(
            "{}_{}",
            (data.latitude * 100.0).round() as i32,
            (data.longitude * 100.0).round() as i32
        );
        let key = CacheKey {
            geohash: geohash.clone(),
            timestamp: data.timestamp.clone(),
        };

        let entry = CacheEntry {
            data: data.clone(),
            created_at: Utc::now(),
            ttl_hours: self.ttl_hours,
        };

        // Store in memory tier
        if let Ok(mut mem) = self.memory.lock() {
            mem.put(key.clone(), entry.clone());
            if let Ok(mut stats) = self.stats.lock() {
                stats.size = mem.len();
            }
        }

        // Store in persistent tier if enabled
        if self.enable_persistence {
            if let Some(db_arc) = &self.db {
                if let Ok(db) = db_arc.lock() {
                    self.put_to_db(&db, &geohash, &data, &entry)?;
                }
            }
        }

        Ok(())
    }

    /// Get weather data for a date range (temporal caching)
    pub fn get_range(
        &self,
        latitude: f64,
        longitude: f64,
        date_range: &DateRange,
    ) -> Vec<EnrichedData> {
        let mut results = Vec::new();

        if let Some(db_arc) = &self.db {
            if let Ok(db) = db_arc.lock() {
                if let Ok(rows) = self.query_range_from_db(&db, latitude, longitude, date_range) {
                    for (data, entry) in rows {
                        if !entry.is_expired() {
                            results.push(data);
                        }
                    }
                    if !results.is_empty() {
                        if let Ok(mut stats) = self.stats.lock() {
                            stats.range_hits += 1;
                        }
                    }
                }
            }
        }

        results
    }

    /// Find nearby cached entries within proximity radius
    fn find_nearby_cached(
        &self,
        latitude: f64,
        longitude: f64,
        timestamp: &str,
    ) -> Option<EnrichedData> {
        let proximity = LocationProximity::new(latitude, longitude, self.proximity_radius_km);

        // Check memory cache for nearby entries
        if let Ok(mem) = self.memory.lock() {
            for entry in mem.iter() {
                if entry.0.timestamp == timestamp && !entry.1.is_expired() {
                    // Extract lat/lon from geohash lookup or from the data
                    if proximity.is_nearby(entry.1.data.latitude, entry.1.data.longitude) {
                        return Some(entry.1.data.clone());
                    }
                }
            }
        }

        // Check persistent cache for nearby entries
        if let Some(db_arc) = &self.db {
            if let Ok(db) = db_arc.lock() {
                if let Ok(nearby) = self.find_nearby_in_db(&db, latitude, longitude, timestamp) {
                    if !nearby.1.is_expired() {
                        if let Ok(mut mem) = self.memory.lock() {
                            let geohash = format!(
                                "{}_{}",
                                (nearby.0.latitude * 100.0).round() as i32,
                                (nearby.0.longitude * 100.0).round() as i32
                            );
                            let key = CacheKey {
                                geohash,
                                timestamp: timestamp.to_string(),
                            };
                            mem.put(key, nearby.1.clone());
                        }
                        return Some(nearby.0);
                    }
                }
            }
        }

        None
    }

    /// Deduplicate batch requests before API calls
    pub fn deduplicate_batch(
        &self,
        requests: &[(String, f64, f64, String)], // (location, lat, lon, timestamp)
    ) -> (Vec<usize>, usize) {
        let mut missing_indices = Vec::new();
        let mut cache_hits = 0;

        for (idx, (location, lat, lon, timestamp)) in requests.iter().enumerate() {
            if self.get(location, *lat, *lon, timestamp).is_none() {
                missing_indices.push(idx);
            } else {
                cache_hits += 1;
            }
        }

        if let Ok(mut stats) = self.stats.lock() {
            stats.deduplication_saves += cache_hits;
        }

        (missing_indices, cache_hits)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear expired entries from database
    pub fn cleanup_expired(&self) -> Result<usize> {
        if let Some(db_arc) = &self.db {
            if let Ok(db) = db_arc.lock() {
                let now = Utc::now();
                let deleted = db.execute(
                    "DELETE FROM weather_cache WHERE
                     datetime(created_at) <= datetime(?, '-' || ttl_hours || ' hours')",
                    params![now.to_rfc3339()],
                )?;
                Ok(deleted)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    // ===== Database Helper Methods =====

    fn get_from_db(
        &self,
        db: &Connection,
        key: &CacheKey,
    ) -> Result<Option<(EnrichedData, CacheEntry)>> {
        let mut stmt = db.prepare(
            "SELECT latitude, longitude, temperature, humidity, condition, location,
                    created_at, ttl_hours
             FROM weather_cache WHERE geohash = ? AND timestamp = ?",
        )?;

        let result = stmt
            .query_row(params![&key.geohash, &key.timestamp], |row| {
                let created_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)
                    .map(|dt| dt.with_timezone(&Utc))?;

                Ok((
                    EnrichedData {
                        location: row.get(5)?,
                        latitude: row.get(0)?,
                        longitude: row.get(1)?,
                        temperature: row.get(2)?,
                        humidity: row.get(3)?,
                        condition: row.get(4)?,
                        timestamp: key.timestamp.clone(),
                    },
                    CacheEntry {
                        data: EnrichedData {
                            location: row.get(5)?,
                            latitude: row.get(0)?,
                            longitude: row.get(1)?,
                            temperature: row.get(2)?,
                            humidity: row.get(3)?,
                            condition: row.get(4)?,
                            timestamp: key.timestamp.clone(),
                        },
                        created_at,
                        ttl_hours: row.get(7)?,
                    },
                ))
            })
            .optional()?;

        Ok(result)
    }

    fn put_to_db(
        &self,
        db: &Connection,
        geohash: &str,
        data: &EnrichedData,
        entry: &CacheEntry,
    ) -> Result<()> {
        db.execute(
            "INSERT OR REPLACE INTO weather_cache
             (geohash, latitude, longitude, timestamp, temperature, humidity,
              condition, location, created_at, ttl_hours)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                geohash,
                data.latitude,
                data.longitude,
                &data.timestamp,
                data.temperature,
                data.humidity,
                &data.condition,
                &data.location,
                entry.created_at.to_rfc3339(),
                entry.ttl_hours,
            ],
        )?;
        Ok(())
    }

    fn query_range_from_db(
        &self,
        db: &Connection,
        latitude: f64,
        longitude: f64,
        date_range: &DateRange,
    ) -> Result<Vec<(EnrichedData, CacheEntry)>> {
        let mut stmt = db.prepare(
            "SELECT latitude, longitude, temperature, humidity, condition, location,
                    timestamp, created_at, ttl_hours
             FROM weather_cache
             WHERE timestamp BETWEEN ? AND ? AND latitude BETWEEN ? AND ?
               AND longitude BETWEEN ? AND ?
             ORDER BY timestamp",
        )?;

        let proximity = LocationProximity::new(latitude, longitude, self.proximity_radius_km);
        let lat_delta = 0.05; // ~5km in degrees
        let lon_delta = 0.05;

        let results = stmt
            .query_map(
                params![
                    date_range.start.to_rfc3339(),
                    date_range.end.to_rfc3339(),
                    latitude - lat_delta,
                    latitude + lat_delta,
                    longitude - lon_delta,
                    longitude + lon_delta,
                ],
                |row| {
                    let db_lat: f64 = row.get(0)?;
                    let db_lon: f64 = row.get(1)?;

                    if !proximity.is_nearby(db_lat, db_lon) {
                        return Ok(None);
                    }

                    let created_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)
                        .map(|dt| dt.with_timezone(&Utc))?;

                    Ok(Some((
                        EnrichedData {
                            location: row.get(5)?,
                            latitude: db_lat,
                            longitude: db_lon,
                            temperature: row.get(2)?,
                            humidity: row.get(3)?,
                            condition: row.get(4)?,
                            timestamp: row.get(6)?,
                        },
                        CacheEntry {
                            data: EnrichedData {
                                location: row.get(5)?,
                                latitude: db_lat,
                                longitude: db_lon,
                                temperature: row.get(2)?,
                                humidity: row.get(3)?,
                                condition: row.get(4)?,
                                timestamp: row.get(6)?,
                            },
                            created_at,
                            ttl_hours: row.get(8)?,
                        },
                    )))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().flatten().collect())
    }

    fn find_nearby_in_db(
        &self,
        db: &Connection,
        latitude: f64,
        longitude: f64,
        timestamp: &str,
    ) -> Result<(EnrichedData, CacheEntry)> {
        let proximity = LocationProximity::new(latitude, longitude, self.proximity_radius_km);
        let lat_delta = 0.05;
        let lon_delta = 0.05;

        let mut stmt = db.prepare(
            "SELECT latitude, longitude, temperature, humidity, condition, location,
                    created_at, ttl_hours
             FROM weather_cache
             WHERE timestamp = ? AND latitude BETWEEN ? AND ?
               AND longitude BETWEEN ? AND ?
             LIMIT 1",
        )?;

        let result = stmt.query_row(
            params![
                timestamp,
                latitude - lat_delta,
                latitude + lat_delta,
                longitude - lon_delta,
                longitude + lon_delta,
            ],
            |row| {
                let db_lat: f64 = row.get(0)?;
                let db_lon: f64 = row.get(1)?;

                if !proximity.is_nearby(db_lat, db_lon) {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }

                let created_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)
                    .map(|dt| dt.with_timezone(&Utc))?;

                Ok((
                    EnrichedData {
                        location: row.get(5)?,
                        latitude: db_lat,
                        longitude: db_lon,
                        temperature: row.get(2)?,
                        humidity: row.get(3)?,
                        condition: row.get(4)?,
                        timestamp: timestamp.to_string(),
                    },
                    CacheEntry {
                        data: EnrichedData {
                            location: row.get(5)?,
                            latitude: db_lat,
                            longitude: db_lon,
                            temperature: row.get(2)?,
                            humidity: row.get(3)?,
                            condition: row.get(4)?,
                            timestamp: timestamp.to_string(),
                        },
                        created_at,
                        ttl_hours: row.get(7)?,
                    },
                ))
            },
        )?;

        Ok(result)
    }
}
