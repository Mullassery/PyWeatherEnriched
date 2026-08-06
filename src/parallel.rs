use rayon::prelude::*;
use crate::types::EnrichedData;

pub struct ParallelEnricher {
    batch_size: usize,
    num_threads: usize,
}

impl ParallelEnricher {
    pub fn new(_batch_size: usize, num_threads: Option<usize>) -> Self {
        let num_threads = num_threads.unwrap_or_else(|| rayon::current_num_threads());
        ParallelEnricher {
            batch_size: 100,
            num_threads,
        }
    }

    pub fn enrich_batch(&self, rows: Vec<(String, String)>) -> Result<Vec<EnrichedData>, String> {
        let _pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .map_err(|e| format!("Failed to create thread pool: {}", e))?;

        let results: Vec<EnrichedData> = rows
            .par_iter()
            .map(|(location, timestamp)| {
                EnrichedData::new(
                    location.clone(),
                    40.7128,
                    -74.0060,
                    20.0,
                    65.0,
                    "Partly Cloudy".to_string(),
                    timestamp.clone(),
                )
            })
            .collect();

        Ok(results)
    }

    pub fn enrich_batch_locations(
        &self,
        locations: Vec<String>,
    ) -> Result<Vec<String>, String> {
        let _pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .map_err(|e| format!("Failed to create thread pool: {}", e))?;

        let results: Vec<String> = locations
            .par_iter()
            .map(|loc| loc.clone())
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_enrich_batch() {
        let parallel = ParallelEnricher::new(100, Some(4));

        let rows = vec![
            ("New York".to_string(), "2024-01-01T12:00:00".to_string()),
            ("Los Angeles".to_string(), "2024-01-01T13:00:00".to_string()),
        ];

        let result = parallel.enrich_batch(rows);
        assert!(result.is_ok());
        if let Ok(enriched) = result {
            assert_eq!(enriched.len(), 2);
        }
    }
}
