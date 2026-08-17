//! Python-facing fluent builder for configuring a `WeatherEnricher`.
//!
//! ```python
//! enricher = (
//!     EnrichmentBuilder()
//!     .with_cache_size(5000)
//!     .build()
//! )
//! ```

use pyo3::prelude::*;

#[pyclass(name = "EnrichmentBuilder")]
#[derive(Clone)]
pub struct EnrichmentBuilder {
    cache_size: usize,
}

#[pymethods]
impl EnrichmentBuilder {
    #[new]
    fn new() -> Self {
        EnrichmentBuilder { cache_size: 1000 }
    }

    /// Set the enricher's in-memory cache capacity (default: 1000 entries).
    fn with_cache_size(&self, size: usize) -> Self {
        EnrichmentBuilder { cache_size: size }
    }

    /// Build a configured `WeatherEnricher`.
    fn build(&self) -> crate::PyWeatherEnricher {
        crate::PyWeatherEnricher::new(Some(self.cache_size))
    }

    fn __repr__(&self) -> String {
        format!("EnrichmentBuilder(cache_size={})", self.cache_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_and_with_cache_size_are_immutable_builder_steps() {
        let base = EnrichmentBuilder::new();
        assert_eq!(base.cache_size, 1000);

        let configured = base.with_cache_size(42);
        assert_eq!(configured.cache_size, 42);
        // Original is untouched (fluent builder returns a new value).
        assert_eq!(base.cache_size, 1000);
    }
}
