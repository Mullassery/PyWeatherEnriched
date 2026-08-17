"""PyWeatherEnriched - hyperlocal weather enrichment + climate feature engineering.

`WeatherEnricher`, `EnrichedRow`, `EnhancedCache`, `CacheStats`, and
`EnrichmentBuilder` are real classes backed by a compiled Rust core: forward
geocoding via OpenStreetMap's Nominatim and historical weather via
Open-Meteo's Archive API, both genuinely fetched over the network (no
fabricated/formula-generated data), with in-memory and optional
SQLite-backed caching.

`features` (and the re-exported `enrich_dataframe`) provide pandas/numpy
climate feature engineering — rolling aggregates, heating/cooling
degree-days, cyclical time encoding, and anomaly z-scores — on top of that
real weather data.
"""

from pyweatherenriched._pyweatherenriched import (
    CacheStats,
    EnhancedCache,
    EnrichedRow,
    EnrichmentBuilder,
    WeatherEnricher,
    __version__,
)

from pyweatherenriched import features
from pyweatherenriched.features import (
    add_anomaly_features,
    add_cyclical_time_features,
    add_degree_days,
    add_rolling_features,
    build_features,
    enrich_dataframe,
)

__all__ = [
    "WeatherEnricher",
    "EnrichedRow",
    "EnhancedCache",
    "CacheStats",
    "EnrichmentBuilder",
    "features",
    "enrich_dataframe",
    "add_rolling_features",
    "add_degree_days",
    "add_cyclical_time_features",
    "add_anomaly_features",
    "build_features",
    "__version__",
]
