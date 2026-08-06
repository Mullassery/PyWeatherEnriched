use pyo3::prelude::*;

mod enricher;
mod geocoder;
mod cache;
mod types;
mod enhanced_cache;
mod python_bindings;
mod parallel;
mod batch_resolver;
mod streaming_io;
mod database;

pub use enricher::WeatherEnricher;
pub use geocoder::Geocoder;
pub use cache::Cache;
pub use enhanced_cache::{EnhancedCache, LocationProximity, DateRange, CacheStats};
pub use python_bindings::EnrichmentBuilder;
pub use parallel::ParallelEnricher;
pub use batch_resolver::{BatchResolver, BatchResolutionResult, DeduplicationStats};
pub use streaming_io::{StreamingReader, StreamingWriter, DataRow};
pub use database::{DatabaseConfig, DatabaseType, DatabaseWriter, DatabasePool, EnrichedRecord};

#[pymodule]
fn pyweatherenriched(_py: Python, m: &pyo3::Bound<pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<PyWeatherEnricher>()?;
    m.add_class::<PyEnrichedRow>()?;
    m.add_class::<PyEnhancedCache>()?;
    m.add_class::<PyCacheStats>()?;
    m.add_class::<EnrichmentBuilder>()?;
    m.add_class::<python_bindings::PyBatchResolver>()?;
    m.add_class::<python_bindings::PyStreamingReader>()?;
    m.add_class::<python_bindings::PyStreamingWriter>()?;
    m.add("__version__", "0.4.0")?;
    Ok(())
}

#[pyclass(name = "WeatherEnricher")]
pub struct PyWeatherEnricher {
    inner: WeatherEnricher,
}

#[pymethods]
impl PyWeatherEnricher {
    #[new]
    #[pyo3(signature = (cache_size=None))]
    fn new(cache_size: Option<usize>) -> Self {
        PyWeatherEnricher {
            inner: WeatherEnricher::new(cache_size.unwrap_or(1000)),
        }
    }

    fn enrich_row(
        &mut self,
        location: String,
        timestamp: String,
        py: Python,
    ) -> PyResult<PyObject> {
        match self.inner.enrich(&location, &timestamp) {
            Ok(result) => {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("location", result.location)?;
                dict.set_item("latitude", result.latitude)?;
                dict.set_item("longitude", result.longitude)?;
                dict.set_item("temperature", result.temperature)?;
                dict.set_item("humidity", result.humidity)?;
                dict.set_item("condition", result.condition)?;
                dict.set_item("timestamp", result.timestamp)?;
                Ok(dict.into())
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                e.to_string(),
            )),
        }
    }

    fn enrich_batch(
        &mut self,
        rows: Vec<(String, String)>,
        py: Python,
    ) -> PyResult<PyObject> {
        let mut results = Vec::new();
        for (location, timestamp) in rows {
            match self.inner.enrich(&location, &timestamp) {
                Ok(result) => {
                    let dict = pyo3::types::PyDict::new(py);
                    dict.set_item("location", result.location)?;
                    dict.set_item("latitude", result.latitude)?;
                    dict.set_item("longitude", result.longitude)?;
                    dict.set_item("temperature", result.temperature)?;
                    dict.set_item("humidity", result.humidity)?;
                    dict.set_item("condition", result.condition)?;
                    dict.set_item("timestamp", result.timestamp)?;
                    results.push(dict.into_py(py));
                }
                Err(_) => {
                    let dict = pyo3::types::PyDict::new(py);
                    dict.set_item("location", location)?;
                    dict.set_item("error", "Enrichment failed")?;
                    results.push(dict.into_py(py));
                }
            }
        }
        Ok(pyo3::types::PyList::new(py, &results)?.into())
    }

    fn cache_stats(&self, py: Python) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        let stats = self.inner.cache_stats();
        dict.set_item("hits", stats.0)?;
        dict.set_item("misses", stats.1)?;
        dict.set_item("size", stats.2)?;
        Ok(dict.into())
    }
}

#[pyclass(name = "EnrichedRow")]
pub struct PyEnrichedRow {
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub temperature: f64,
    pub humidity: f64,
    pub condition: String,
    pub timestamp: String,
}

#[pyclass(name = "CacheStats")]
pub struct PyCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub proximity_hits: usize,
    pub range_hits: usize,
    pub deduplication_saves: usize,
    pub size: usize,
}

#[pymethods]
impl PyCacheStats {
    fn __repr__(&self) -> String {
        format!(
            "CacheStats(hits={}, misses={}, proximity_hits={}, range_hits={}, dedup_saves={}, size={})",
            self.hits, self.misses, self.proximity_hits, self.range_hits, self.deduplication_saves, self.size
        )
    }

    #[getter]
    fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

#[pyclass(name = "EnhancedCache")]
pub struct PyEnhancedCache {
    inner: EnhancedCache,
}

#[pymethods]
impl PyEnhancedCache {
    #[new]
    #[pyo3(signature = (cache_size=1000, db_path=None))]
    fn new(cache_size: usize, db_path: Option<String>) -> PyResult<Self> {
        let cache = if let Some(path) = db_path {
            EnhancedCache::with_persistence(cache_size, &path)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
        } else {
            EnhancedCache::new(cache_size)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
        };
        Ok(PyEnhancedCache { inner: cache })
    }

    fn set_proximity_radius(&mut self, radius_km: f64) {
        self.inner.set_proximity_radius(radius_km);
    }

    fn set_ttl(&mut self, hours: i64) {
        self.inner.set_ttl(hours);
    }

    fn get(
        &self,
        location: String,
        latitude: f64,
        longitude: f64,
        timestamp: String,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        match self.inner.get(&location, latitude, longitude, &timestamp) {
            Some(data) => {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("location", data.location)?;
                dict.set_item("latitude", data.latitude)?;
                dict.set_item("longitude", data.longitude)?;
                dict.set_item("temperature", data.temperature)?;
                dict.set_item("humidity", data.humidity)?;
                dict.set_item("condition", data.condition)?;
                dict.set_item("timestamp", data.timestamp)?;
                Ok(Some(dict.into()))
            }
            None => Ok(None),
        }
    }

    fn put(&self, location: String, latitude: f64, longitude: f64, temperature: f64, humidity: f64, condition: String, timestamp: String) -> PyResult<()> {
        let data = types::EnrichedData {
            location,
            latitude,
            longitude,
            temperature,
            humidity,
            condition,
            timestamp,
        };
        self.inner.put(data)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn deduplicate_batch(&self, requests: Vec<(String, f64, f64, String)>) -> PyResult<(Vec<usize>, usize)> {
        let (missing, hits) = self.inner.deduplicate_batch(&requests);
        Ok((missing, hits))
    }

    fn stats(&self, py: Python) -> PyResult<PyObject> {
        let s = self.inner.stats();
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("hits", s.hits)?;
        dict.set_item("misses", s.misses)?;
        dict.set_item("proximity_hits", s.proximity_hits)?;
        dict.set_item("range_hits", s.range_hits)?;
        dict.set_item("deduplication_saves", s.deduplication_saves)?;
        dict.set_item("size", s.size)?;
        let total = s.hits + s.misses;
        let hit_ratio = if total == 0 { 0.0 } else { s.hits as f64 / total as f64 };
        dict.set_item("hit_ratio", hit_ratio)?;
        Ok(dict.into())
    }

    fn cleanup_expired(&self) -> PyResult<usize> {
        self.inner.cleanup_expired()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}

