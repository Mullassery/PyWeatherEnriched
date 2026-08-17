//! A minimal in-memory LRU cache of [`EnrichedData`], for callers who don't
//! need [`crate::enhanced_cache::EnhancedCache`]'s proximity/TTL/persistence
//! features.

use crate::types::EnrichedData;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct Cache {
    inner: Mutex<LruCache<(String, String), EnrichedData>>,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).expect("capacity >= 1 after max(1)");
        Cache {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn get(&self, location: &str, timestamp: &str) -> Option<EnrichedData> {
        let key = (location.to_string(), timestamp.to_string());
        self.inner.lock().ok()?.get(&key).cloned()
    }

    pub fn put(&self, data: EnrichedData) {
        let key = (data.location.clone(), data.timestamp.clone());
        if let Ok(mut inner) = self.inner.lock() {
            inner.put(key, data);
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(location: &str, timestamp: &str) -> EnrichedData {
        EnrichedData {
            location: location.to_string(),
            latitude: 0.0,
            longitude: 0.0,
            temperature: 20.0,
            humidity: 50.0,
            condition: "Clear".to_string(),
            timestamp: timestamp.to_string(),
        }
    }

    #[test]
    fn test_put_then_get_round_trips() {
        let cache = Cache::new(10);
        cache.put(sample("NYC", "t1"));
        let got = cache.get("NYC", "t1").unwrap();
        assert_eq!(got.location, "NYC");
    }

    #[test]
    fn test_miss_returns_none() {
        let cache = Cache::new(10);
        assert!(cache.get("Nowhere", "t1").is_none());
    }

    #[test]
    fn test_evicts_least_recently_used_at_capacity() {
        let cache = Cache::new(1);
        cache.put(sample("A", "t1"));
        cache.put(sample("B", "t1")); // evicts A

        assert!(cache.get("A", "t1").is_none());
        assert!(cache.get("B", "t1").is_some());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear_empties_cache() {
        let cache = Cache::new(10);
        cache.put(sample("A", "t1"));
        cache.clear();
        assert!(cache.is_empty());
    }
}
