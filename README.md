# PyWeatherEnriched

**Real geocoding + real historical weather, wired into a Rust core with pandas/numpy climate feature engineering on top.**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![PyPI](https://img.shields.io/pypi/v/pyweatherenriched)](https://pypi.org/project/pyweatherenriched/)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10%2B-blue)](https://www.python.org)
[![Tests](https://github.com/Mullassery/PyWeatherEnriched/actions/workflows/tests.yml/badge.svg)](https://github.com/Mullassery/PyWeatherEnriched/actions/workflows/tests.yml)

## What it is

Given a location name and a timestamp, PyWeatherEnriched geocodes the location
(OpenStreetMap Nominatim) and looks up the real, genuinely observed historical weather for
that place and hour (Open-Meteo's free Archive API) — no formula-generated or fabricated
numbers. The lookup/geocoding core is Rust (via PyO3) for speed and a small memory
footprint; a Python layer on top adds pandas/numpy feature engineering (rolling aggregates,
heating/cooling degree-days, cyclical time encoding, anomaly z-scores) useful for feeding
weather into an ML pipeline.

## Features

- **Real historical weather.** Open-Meteo Archive API — observed data, not forecasts or
  formulas.
- **Real forward geocoding.** OpenStreetMap Nominatim turns a location name into
  coordinates for you.
- **Historical backfill.** `enrich_range(location, start_date, end_date)` geocodes once and
  fetches every hourly observation across the whole range in a single Open-Meteo request.
- **pandas/numpy feature engineering** via standalone functions or the `build_features`
  convenience pipeline: degree-days (`hdd`/`cdd`), cyclical hour/day-of-week/day-of-year
  encodings, trailing rolling aggregates, and anomaly z-scores.
- **Two-tier caching.** A built-in in-memory LRU cache, plus an opt-in SQLite-backed
  `EnhancedCache` with geospatial-proximity matching, TTL expiration, batch deduplication,
  and date-range queries.
- **Honest failure handling.** Rows whose lookup fails get `NaN` rather than raising or
  fabricating a value; hours with no observation are omitted, not filled.

## Who it's for

- **ML/data engineers enriching a training dataset with real historical weather** for a
  location/timestamp pair, instead of a synthetic or averaged fill-in.
- **Teams building weather-sensitive models** that need ready-made climate features
  (rolling aggregates, degree-days, cyclical encodings, anomaly z-scores).
- **Anyone who wants Open-Meteo data without writing the geocoding and feature math
  themselves.**

## What it is not

- **Not real-time or forecast weather.** This is historical, observed data only.
- **Not a source of hyperlocal data beyond what Open-Meteo provides.** The package layers
  geocoding and feature engineering on top of the same public archive; it does not produce
  new weather measurements.
- **Geospatial building blocks are only partly exposed.** `lapse_rate_adjustment()`,
  `terrain_roughness()`, `ReverseGeocoder`, and `UrbanHeatIsland` are available from
  Python. The two classes read OSM GeoJSON tiles from a local directory you pass in;
  they raise `FileNotFoundError`/`RuntimeError` when a tile is missing or has no usable
  feature instead of fabricating an address or value. SRTM tile lookup itself is still
  internal only.
- **UHI is a model estimate, not an observation.** `UrbanHeatIsland` computes building
  density/height from real OSM footprints and applies the project's documented formula;
  it does not measure actual urban heat.
- **Optional layers are GeoJSON-backed.** `VegetationLayer`, `SoilLayer`, and
  `FloodRiskLayer` read user-supplied GeoJSON tiles and return the properties present
  in the tile (`ndvi`/`vegetation_type`, `texture`/`water_holding_capacity_mm`/`ph`,
  `risk_score`/`risk_category`). Missing tiles or required properties raise an error
  rather than fabricating values. Google Maps/USPS reverse geocoding remain
  unimplemented; the OSM/Nominatim path is the supported one.
- **Not fully covered by `cargo test`.** Plain `cargo test` (without `--lib`) currently
  fails to compile because a stale integration test references source files not declared as
  `mod`s in `src/lib.rs`. Use `cargo test --lib`.

## Project status

**Beta.** Version 0.7.0. The Python-facing geocoding, historical weather, caching,
feature-engineering, and local-tile reverse-geocoding/UHI paths are real and tested;
the remaining Rust building blocks above are implemented but not exposed. Changes are
tracked in [CHANGELOG.md](CHANGELOG.md); see also
[`docs/internal/ROADMAP_HONEST.md`](docs/internal/ROADMAP_HONEST.md).

## Installation

```bash
pip install pyweatherenriched
```

Requires Python 3.10+. CI builds prebuilt wheels for Linux (x86_64/ARM64), macOS
(Intel/Apple Silicon), and Windows (x86_64) on every release, but the CI→PyPI publish step
is currently broken (PyPI rejects the upload with `403 Invalid or non-existent
authentication information` — a stale/invalid API token), so **only a macOS ARM64 `abi3`
wheel plus an sdist is actually published on PyPI today**. On any other platform,
`pip install` falls back to building the sdist from source, which does need a Rust toolchain
(see Requirements below).

## Quick start

```python
import pyweatherenriched as pwe

enricher = pwe.WeatherEnricher()
result = enricher.enrich_row("New York", "2024-06-15T12:00:00")

print(result)
# {'location': 'New York', 'latitude': 40.7127281, 'longitude': -74.0060152,
#  'temperature': 19.4, 'humidity': 74.0, 'condition': 'Clear',
#  'timestamp': '2024-06-15T12:00:00'}
```

`enrich_row` makes two real network calls the first time it sees a location/timestamp pair
(one to Nominatim to geocode `location`, one to Open-Meteo for the historical weather at
those coordinates); the in-process cache means repeating the same lookup doesn't hit the
network again:

```python
fresh = pwe.WeatherEnricher()
fresh.enrich_row("New York", "2024-06-15T12:00:00")  # network
fresh.enrich_row("New York", "2024-06-15T12:00:00")  # cache hit
print(fresh.cache_stats())  # {'hits': 1, 'misses': 1, 'size': 1}
```

## Offline climate features

Add climatological normals/anomalies and common indices without another
network call:

```python
from pyweatherenriched.features import (
    add_climate_normals, add_heat_index, add_growing_degree_days,
)

df = add_climate_normals(enriched_df, group_col="location")
df = add_heat_index(df)
df = add_growing_degree_days(df, base_c=10)
```

`add_climate_normals` computes a centered day-of-year normal per location
(±15 days by default) and adds temperature normal/anomaly columns. Heat index
and GDD are standard formulas; all functions are offline and deterministic.

## Usage

### Historical backfill

`enrich_row`/`enrich_batch` fetch one day of data per call. To backfill a date range for a
location, `enrich_range` geocodes once and fetches every hourly observation across the whole
range in a single Open-Meteo request:

```python
enricher = pwe.WeatherEnricher()
rows = enricher.enrich_range("Chicago", "2024-06-01", "2024-06-07")
print(len(rows))  # ~168 (7 days x 24 hours, minus any hours Open-Meteo has no data for)
```

Hours Open-Meteo has no observation for (e.g. the tail of a range that runs past the latest
available data) are omitted from the result rather than filled with a fabricated value.

### Feature engineering on a DataFrame

`enrich_dataframe` bridges the Rust core to pandas — fetch real weather for every row of a
DataFrame in one call — and the `features` functions build standard climate ML features on
top of the result:

```python
import pandas as pd
import pyweatherenriched as pwe

orders = pd.DataFrame({
    "order_id": ["A1", "A2", "A3"],
    "location": ["Chicago", "Miami", "Denver"],
    "timestamp": ["2024-01-15T12:00:00", "2024-06-15T12:00:00", "2024-03-15T12:00:00"],
})

enricher = pwe.WeatherEnricher()
enriched = pwe.enrich_dataframe(enricher, orders)     # + latitude/longitude/temperature/humidity/condition
features = pwe.build_features(enriched)                # + hdd/cdd, cyclical time, rolling stats, anomaly z-score

print(features[["order_id", "temperature", "hdd", "cdd", "temperature_zscore"]])
```

Rows whose lookup fails (unresolvable location, upstream error) get `NaN` in the new columns
instead of raising or fabricating a value, so one bad row doesn't lose the batch.

`build_features` is a convenience pipeline; each step is also a standalone function:

| Function | What it adds |
|---|---|
| `add_degree_days(df, temp_col, base_temp=18.0)` | `hdd`, `cdd` — heating/cooling degree-days (`max(0, base - t)` / `max(0, t - base)`) |
| `add_cyclical_time_features(df, timestamp_col)` | `hour_sin/cos`, `day_of_week_sin/cos`, `day_of_year_sin/cos` — sin/cos encoding so e.g. 23:59 and 00:00 stay numerically adjacent |
| `add_rolling_features(df, value_col, window, group_col=None, stats=(...))` | trailing rolling aggregates (mean/std/min/max/...), optionally computed independently per group |
| `add_anomaly_features(df, value_col, group_col=None, baseline="expanding"\|"rolling"\|"global")` | a z-score measuring how unusual each reading is relative to a chosen baseline |

### Caching

`WeatherEnricher` has a small built-in LRU cache. For larger workloads, `EnhancedCache` adds
a second, SQLite-backed persistent tier with geospatial-proximity matching, TTL expiration,
batch deduplication, and date-range queries:

```python
from pyweatherenriched import EnhancedCache

cache = EnhancedCache(cache_size=5000, db_path="weather_cache.db")
cache.set_proximity_radius(10.0)  # treat lookups within 10km as cache hits
cache.set_ttl(72)                 # hours before an entry expires

cache.put("New York", 40.7128, -74.0060, 15.2, 65.0, "Partly Cloudy", "2024-01-15T12:00:00Z")
result = cache.get("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z")

# Deduplicate a batch before making any API calls.
batch = [
    ("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z"),
    ("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z"),  # duplicate
]
missing_indices, cache_hits = cache.deduplicate_batch(batch)

# Every observation in a date range near a point (requires db_path).
rows = cache.get_range(40.7128, -74.0060, "2024-01-01T00:00:00Z", "2024-01-31T23:59:59Z")

stats = cache.stats()
print(f"hit ratio: {stats['hit_ratio']:.1%}")
```

`get()` checks the memory tier, then the persistent tier (if `db_path` was given), then
falls back to a proximity search; `get_range` only queries the persistent tier, so it always
returns `[]` without `db_path`.

### Local OSM reverse geocoding and UHI

`ReverseGeocoder` and `UrbanHeatIsland` read OSM-derived GeoJSON tiles from a directory
you provide. The default pattern is `{lat}_{lon}.geojson`, where `lat`/`lon` are
`floor(latitude)`/`floor(longitude)` (e.g. `40_-74.geojson`):

```python
from pyweatherenriched import ReverseGeocoder, UrbanHeatIsland

geocoder = ReverseGeocoder("/data/osm")
geocoder.reverse_geocode(40.7128, -74.006)
# {'postal_code': '10001', 'street_address': None, 'city': 'New York',
#  'state': 'NY', 'country': 'US', 'country_code': 'US', ..., 'source': 'osm'}

uhi = UrbanHeatIsland("/data/osm")
uhi.get_uhi_data(40.7484, -73.9857)
# {'building_density_percent': ..., 'average_building_height_m': ...,
#  'uhi_effect_c': ..., 'location_type': ...}
```

Reverse geocoding returns the nearest feature's real OSM address properties (`confidence`
decays with distance). `UrbanHeatIsland` is a formula-based estimate from real building
footprint area/height, not a measured temperature. Both raise `FileNotFoundError` when the
tile file is absent and `RuntimeError` when the tile has no usable feature/postal code;
`ValueError` is raised for out-of-range coordinates. Optional vegetation/soil/flood layers
and alternative commercial geocoding providers remain deliberately unimplemented and are
not exposed.

### vs raw Open-Meteo

PyWeatherEnriched is built directly on Open-Meteo's free Archive API, so the honest
comparison isn't "who has better weather data" (same real source) — it's "what do you get
for the extra network round-trip and processing time." Tested live against real data for 3
real location/timestamp pairs (Chicago/Jan, Miami/Jun, Denver/Mar 2024, noon local):

| | Raw Open-Meteo (`openmeteo-requests`, direct) | PyWeatherEnriched (`enrich_dataframe` + `build_features`) |
|---|---|---|
| Input required | Real lat/lon (you geocode yourself) | Real location name — geocodes internally (Nominatim) |
| Fields returned | `temperature`, `humidity` (2 raw fields) | `+ condition`, `+ hdd`/`cdd`, cyclical hour/day/day-of-year encodings, rolling mean/std, anomaly z-score (11 engineered features on top) |
| Time, 3 locations, cold cache | 1.26s | 2.63s (geocoding is a second real network call raw Open-Meteo alone doesn't need) |
| Missing-data handling | Raises/errors on a bad request | Failed rows get `NaN`, not fabricated values |

Correctness was verified by hand against real output: `hdd`/`cdd` matched
`max(0, 18-t)`/`max(0, t-18)` exactly for all 3 real temperatures, and `hour_sin`/`hour_cos`
matched `sin`/`cos(2π·hour/24)` exactly. Returned temperature/humidity also matched the raw
Open-Meteo baseline for the same coordinates/timestamps (rounding only).

### Requirements

- Python 3.10+
- Rust 1.75+ (only if building from source — not needed to `pip install` on a platform with
  a published wheel)
- Network access to `nominatim.openstreetmap.org` and `archive-api.open-meteo.com` at call
  time (no API key needed for either)

## Documentation

- [BUILD.md](BUILD.md) — wheel-building/release details.
- [docs/internal/ROADMAP_HONEST.md](docs/internal/ROADMAP_HONEST.md) — what's real, what's deferred, and why.
- [CHANGELOG.md](CHANGELOG.md) — release history.
- [docs/](docs/) — additional documentation.

## Known issues and deferred work

**Blocked / failed:**
- Expose implemented Rust features; fix cargo-test build.

**Rejected / deferred:**
- Forecasts/realtime; thin wrapper.

Full shipped/blocked/rejected detail: [`docs/internal/QUEUE_STATUS.md`](docs/internal/QUEUE_STATUS.md).

## Contributing

```bash
git clone https://github.com/Mullassery/PyWeatherEnriched.git
cd PyWeatherEnriched

maturin develop --release   # build the Rust extension + install editable
cargo test --lib            # Rust unit tests
pip install -e ".[dev]"
pytest tests/ -v            # Python tests, incl. live-network ones
```

`cargo run --bin pyweatherenriched-validate` is a small smoke-test CLI that exercises real
geocoding + weather fetch end to end — useful to confirm a build/environment can actually
reach Nominatim and Open-Meteo.

## Related projects

Other tools from the same family, for adjacent parts of the data/ML stack:

- [PySynthData](https://github.com/Mullassery/PySynthData) — synthetic data generation for ML training: realistic datasets for model development and testing.
- [StatGuardian](https://github.com/Mullassery/StatGuardian) — fast, declarative data quality framework (Rust): schema validation, drift detection, anomaly detection.
- [PyReverseETL](https://github.com/Mullassery/PyReverseETL) — reverse ETL runtime: move data into CRMs and marketing tools with lineage and compliance enforcement.
- [PyDBTGuard](https://github.com/Mullassery/PyDBTGuard) — analyzes a dbt project's `manifest.json` to flag risky tests, map blast radius, and estimate test cost.
- [PyAirflowTester](https://github.com/Mullassery/PyAirflowTester) — dependency intelligence and quality assurance for Airflow and dbt pipelines.

[Browse all projects →](https://github.com/Mullassery?tab=repositories)

## License

This project is licensed under the [Apache License 2.0](LICENSE).
