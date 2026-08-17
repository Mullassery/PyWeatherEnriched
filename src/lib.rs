use pyo3::prelude::*;

mod cache;
mod enhanced_cache;
mod enricher;
mod geocoder;
mod python_bindings;
mod types;
// Elevation/UHI/reverse-geocoding/data-source building blocks. These are
// real, independently unit-tested (see each submodule's `#[cfg(test)]`),
// but are NOT wired into the public Python API below and NOT reachable
// from anywhere else in the crate yet — so they're honestly `#[allow(dead_code)]`
// rather than pretending to be shipped functionality. Do not describe this
// module's features as available to Python users until it is actually
// wired into a `#[pymodule]` class and covered by an end-to-end test.
#[allow(dead_code)]
mod geospatial;

pub use cache::Cache;
pub use enhanced_cache::{CacheStats, DateRange, EnhancedCache, LocationProximity};
pub use enricher::WeatherEnricher;
pub use geocoder::Geocoder;
pub use python_bindings::EnrichmentBuilder;

// The compiled name here (`_pyweatherenriched`) must match the last path
// component of `[tool.maturin] module-name` in pyproject.toml
// ("pyweatherenriched._pyweatherenriched"): Python's import machinery looks
// for a `PyInit_<last-component>` symbol. The pure-Python
// `python/pyweatherenriched/__init__.py` re-exports the real classes from
// this native submodule, giving the package a normal mixed Rust/Python
// (maturin) layout instead of the native module masquerading as the
// top-level package.
#[pymodule]
fn _pyweatherenriched(_py: Python, m: &pyo3::Bound<pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<PyWeatherEnricher>()?;
    m.add_class::<PyEnrichedRow>()?;
    m.add_class::<PyEnhancedCache>()?;
    m.add_class::<PyCacheStats>()?;
    m.add_class::<EnrichmentBuilder>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
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

    fn enrich_batch(&mut self, rows: Vec<(String, String)>, py: Python) -> PyResult<PyObject> {
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
                    results.push(dict.into_any().unbind());
                }
                Err(_) => {
                    let dict = pyo3::types::PyDict::new(py);
                    dict.set_item("location", location)?;
                    dict.set_item("error", "Enrichment failed")?;
                    results.push(dict.into_any().unbind());
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

/// A typed, constructible alternative to the plain dicts `WeatherEnricher`
/// methods return — useful when callers want an object with named
/// attributes (e.g. to build one up field-by-field) rather than a dict
/// literal.
#[pyclass(name = "EnrichedRow")]
#[derive(Clone)]
pub struct PyEnrichedRow {
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub latitude: f64,
    #[pyo3(get)]
    pub longitude: f64,
    #[pyo3(get)]
    pub temperature: f64,
    #[pyo3(get)]
    pub humidity: f64,
    #[pyo3(get)]
    pub condition: String,
    #[pyo3(get)]
    pub timestamp: String,
}

#[pymethods]
impl PyEnrichedRow {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        location: String,
        latitude: f64,
        longitude: f64,
        temperature: f64,
        humidity: f64,
        condition: String,
        timestamp: String,
    ) -> Self {
        PyEnrichedRow {
            location,
            latitude,
            longitude,
            temperature,
            humidity,
            condition,
            timestamp,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "EnrichedRow(location={:?}, latitude={}, longitude={}, temperature={}, humidity={}, condition={:?}, timestamp={:?})",
            self.location, self.latitude, self.longitude, self.temperature, self.humidity, self.condition, self.timestamp
        )
    }
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
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
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

    // One argument per `EnrichedData` field mirrors the Python-facing
    // `get(...)` above and keeps the call site a flat, self-explanatory
    // list of scalars rather than requiring callers to build an
    // intermediate object first.
    #[allow(clippy::too_many_arguments)]
    fn put(
        &self,
        location: String,
        latitude: f64,
        longitude: f64,
        temperature: f64,
        humidity: f64,
        condition: String,
        timestamp: String,
    ) -> PyResult<()> {
        let data = types::EnrichedData {
            location,
            latitude,
            longitude,
            temperature,
            humidity,
            condition,
            timestamp,
        };
        self.inner
            .put(data)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Fetch every cached observation near (`latitude`, `longitude`) whose
    /// timestamp falls within `[start, end]` (RFC3339 timestamps, e.g.
    /// `"2024-06-01T00:00:00Z"`).
    ///
    /// Only the SQLite-backed persistent tier supports range queries, so
    /// this always returns `[]` for a cache constructed without
    /// `db_path=...`.
    fn get_range(
        &self,
        latitude: f64,
        longitude: f64,
        start: String,
        end: String,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let start_dt = chrono::DateTime::parse_from_rfc3339(&start)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid start timestamp {start:?}: {e}"
                ))
            })?
            .with_timezone(&chrono::Utc);
        let end_dt = chrono::DateTime::parse_from_rfc3339(&end)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid end timestamp {end:?}: {e}"
                ))
            })?
            .with_timezone(&chrono::Utc);
        let range = DateRange::new(start_dt, end_dt);

        let mut out = Vec::new();
        for data in self.inner.get_range(latitude, longitude, &range) {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("location", data.location)?;
            dict.set_item("latitude", data.latitude)?;
            dict.set_item("longitude", data.longitude)?;
            dict.set_item("temperature", data.temperature)?;
            dict.set_item("humidity", data.humidity)?;
            dict.set_item("condition", data.condition)?;
            dict.set_item("timestamp", data.timestamp)?;
            out.push(dict.into_any().unbind());
        }
        Ok(out)
    }

    fn deduplicate_batch(
        &self,
        requests: Vec<(String, f64, f64, String)>,
    ) -> PyResult<(Vec<usize>, usize)> {
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
        let hit_ratio = if total == 0 {
            0.0
        } else {
            s.hits as f64 / total as f64
        };
        dict.set_item("hit_ratio", hit_ratio)?;
        Ok(dict.into())
    }

    fn cleanup_expired(&self) -> PyResult<usize> {
        self.inner
            .cleanup_expired()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}
