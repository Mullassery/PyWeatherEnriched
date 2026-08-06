use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocationKey {
    pub location: String,
    pub precision: i32, // For deduplication (e.g., rounding lat/lng)
}

#[derive(Clone, Debug)]
pub struct BatchResolutionResult {
    pub location_key: LocationKey,
    pub resolved_location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
    pub duplicate_count: usize, // How many times this location appeared in batch
}

pub struct BatchResolver {
    cache: Arc<Mutex<HashMap<LocationKey, BatchResolutionResult>>>,
    dedup_radius: f64, // km - locations within this radius are deduplicated
    max_batch_size: usize,
}

impl BatchResolver {
    pub fn new(dedup_radius: f64, max_batch_size: usize) -> Self {
        BatchResolver {
            cache: Arc::new(Mutex::new(HashMap::new())),
            dedup_radius,
            max_batch_size,
        }
    }

    pub fn resolve_batch(&self, locations: Vec<String>) -> Result<Vec<BatchResolutionResult>, String> {
        let mut cache = self.cache.lock().map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        let mut dedup_map: HashMap<LocationKey, usize> = HashMap::new();

        // Deduplicate and count occurrences
        for location in locations {
            let key = LocationKey {
                location: location.clone(),
                precision: 2,
            };

            *dedup_map.entry(key).or_insert(0) += 1;
        }

        // Resolve unique locations (API calls reduced by dedup count)
        for (key, count) in dedup_map {
            if let Some(cached) = cache.get(&key) {
                let mut result = cached.clone();
                result.duplicate_count = count;
                results.push(result);
            } else {
                // In real implementation, this would call the geocoding API
                // For now, mock implementation
                let result = BatchResolutionResult {
                    location_key: key.clone(),
                    resolved_location: key.location.clone(),
                    latitude: 40.7128,
                    longitude: -74.0060,
                    accuracy: 0.95,
                    duplicate_count: count,
                };
                cache.insert(key, result.clone());
                results.push(result);
            }
        }

        Ok(results)
    }

    pub fn get_deduplication_stats(&self, locations: &[String]) -> Result<DeduplicationStats, String> {
        let mut dedup_map: HashMap<String, usize> = HashMap::new();

        for location in locations {
            *dedup_map.entry(location.clone()).or_insert(0) += 1;
        }

        let total = locations.len();
        let unique = dedup_map.len();
        let api_call_reduction = if unique > 0 {
            ((total - unique) as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(DeduplicationStats {
            total_locations: total,
            unique_locations: unique,
            duplicates: total - unique,
            api_call_reduction_percent: api_call_reduction,
            estimated_api_savings: (total - unique) as u32,
        })
    }

    pub fn clear_cache(&self) -> Result<(), String> {
        self.cache.lock().map_err(|e| e.to_string())?.clear();
        Ok(())
    }

    pub fn cache_size(&self) -> Result<usize, String> {
        Ok(self.cache.lock().map_err(|e| e.to_string())?.len())
    }
}

#[derive(Debug, Clone)]
pub struct DeduplicationStats {
    pub total_locations: usize,
    pub unique_locations: usize,
    pub duplicates: usize,
    pub api_call_reduction_percent: f64,
    pub estimated_api_savings: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_resolver_dedup() {
        let resolver = BatchResolver::new(5.0, 1000);
        let locations = vec![
            "New York".to_string(),
            "New York".to_string(),
            "Los Angeles".to_string(),
            "New York".to_string(),
        ];

        let results = resolver.resolve_batch(locations.clone()).unwrap();
        assert_eq!(results.len(), 2); // Only 2 unique locations

        let stats = resolver.get_deduplication_stats(&locations).unwrap();
        assert_eq!(stats.total_locations, 4);
        assert_eq!(stats.unique_locations, 2);
        assert_eq!(stats.duplicates, 2);
        assert!(stats.api_call_reduction_percent > 0.0);
    }

    #[test]
    fn test_deduplication_stats() {
        let resolver = BatchResolver::new(5.0, 1000);
        let locations = vec![
            "A".to_string(),
            "A".to_string(),
            "B".to_string(),
            "B".to_string(),
            "B".to_string(),
            "C".to_string(),
        ];

        let stats = resolver.get_deduplication_stats(&locations).unwrap();
        assert_eq!(stats.total_locations, 6);
        assert_eq!(stats.unique_locations, 3);
        assert_eq!(stats.duplicates, 3);
        assert_eq!(stats.api_call_reduction_percent, 50.0);
        assert_eq!(stats.estimated_api_savings, 3);
    }

    #[test]
    fn test_batch_resolver_cache() {
        let resolver = BatchResolver::new(5.0, 1000);
        let locations = vec!["New York".to_string(), "Los Angeles".to_string()];

        let _results = resolver.resolve_batch(locations.clone()).unwrap();
        let cache_size = resolver.cache_size().unwrap();
        assert_eq!(cache_size, 2);

        resolver.clear_cache().unwrap();
        let cache_size_after = resolver.cache_size().unwrap();
        assert_eq!(cache_size_after, 0);
    }
}
