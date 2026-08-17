# PyWeatherEnriched

Real geocoding + real historical weather, wired into a Rust core with
pandas/numpy climate feature engineering on top.

[![PyPI](https://img.shields.io/pypi/v/pyweatherenriched)](https://pypi.org/project/pyweatherenriched)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10%2B-blue)](https://www.python.org)
[![Tests](https://github.com/Mullassery/PyWeatherEnriched/actions/workflows/tests.yml/badge.svg)](https://github.com/Mullassery/PyWeatherEnriched/actions/workflows/tests.yml)
[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-blue.svg)](./LICENSE)

Given a location name and a timestamp, PyWeatherEnriched geocodes the
location (OpenStreetMap Nominatim) and looks up the real, genuinely observed
historical weather for that place and hour (Open-Meteo's free Archive API)
— no formula-generated or fabricated numbers. The lookup/geocoding core is
Rust (via PyO3) for speed and a small memory footprint; a Python layer on
top adds pandas/numpy feature engineering (rolling aggregates,
heating/cooling degree-days, cyclical time encoding, anomaly z-scores)
useful for feeding weather into an ML pipeline.

## Install

```bash
pip install pyweatherenriched
```

Requires Python 3.10+. Ships as prebuilt wheels for Linux (x86_64/ARM64),
macOS (Intel/Apple Silicon), and Windows (x86_64) — no Rust toolchain
needed to install.

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

`enrich_row` makes two real network calls the first time it sees a
location/timestamp pair (one to Nominatim to geocode `location`, one to
Open-Meteo for the historical weather at those coordinates); the in-process
cache means repeating the same lookup doesn't hit the network again:

```python
fresh = pwe.WeatherEnricher()
fresh.enrich_row("New York", "2024-06-15T12:00:00")  # network
fresh.enrich_row("New York", "2024-06-15T12:00:00")  # cache hit
print(fresh.cache_stats())  # {'hits': 1, 'misses': 1, 'size': 1}
```

## Feature engineering on a DataFrame

`enrich_dataframe` bridges the Rust core to pandas — fetch real weather for
every row of a DataFrame in one call — and the `features` functions build
standard climate ML features on top of the result:

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

Rows whose lookup fails (unresolvable location, upstream error) get `NaN`
in the new columns instead of raising or fabricating a value, so one bad
row doesn't lose the batch.

`build_features` is a convenience pipeline; each step is also a standalone
function you can call individually with more control:

| Function | What it adds |
|---|---|
| `add_degree_days(df, temp_col, base_temp=18.0)` | `hdd`, `cdd` — heating/cooling degree-days (standard energy-demand/agriculture signal: `max(0, base - t)` / `max(0, t - base)`) |
| `add_cyclical_time_features(df, timestamp_col)` | `hour_sin/cos`, `day_of_week_sin/cos`, `day_of_year_sin/cos` — sin/cos encoding so e.g. 23:59 and 00:00 stay numerically adjacent |
| `add_rolling_features(df, value_col, window, group_col=None, stats=(...))` | trailing rolling aggregates (mean/std/min/max/...), optionally computed independently per group (e.g. per location) |
| `add_anomaly_features(df, value_col, group_col=None, baseline="expanding"\|"rolling"\|"global")` | a z-score measuring how unusual each reading is relative to a chosen baseline |

## Caching

`WeatherEnricher` has a small built-in LRU cache. For larger workloads,
`EnhancedCache` adds a second, SQLite-backed persistent tier with
geospatial-proximity matching, TTL expiration, batch deduplication, and
date-range queries:

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

`get()` checks the memory tier, then the persistent tier (if `db_path` was
given), then falls back to a proximity search; `get_range` only queries the
persistent tier, so it always returns `[]` without `db_path`.

## What's real vs. not (yet)

- **Real**: forward geocoding (Nominatim), historical weather (Open-Meteo
  Archive API), in-memory LRU cache, SQLite-backed `EnhancedCache`
  (proximity matching, TTL, dedup, date-range queries), pandas/numpy
  feature engineering.
- **Not yet exposed to Python**: the crate has additional Rust-side,
  independently unit-tested building blocks — elevation/lapse-rate
  adjustment (real SRTM GeoTIFF parsing), urban-heat-island modeling (real
  OSM building-density analysis), and OSM-based reverse geocoding — that
  aren't wired into the Python API yet. They live under `src/geospatial/`
  if you want to build on them.
- **Deliberately unimplemented**: vegetation/NDVI, soil, and flood-risk
  layers, plus additional commercial reverse-geocoding provider backends,
  are framework stubs (`src/geospatial/optional.rs`) that return a clear
  "not yet implemented" error rather than fake data.

## Development

```bash
git clone https://github.com/Mullassery/PyWeatherEnriched.git
cd PyWeatherEnriched

maturin develop --release   # build the Rust extension + install editable
cargo test                  # Rust unit tests (32 tests)
pip install -e ".[dev]"
pytest tests/ -v            # Python tests (33 tests, incl. live-network ones)
```

`cargo run --bin pyweatherenriched-validate` is a small smoke-test CLI that
exercises real geocoding + weather fetch end to end — useful to confirm a
build/environment can actually reach Nominatim and Open-Meteo.

See [BUILD.md](BUILD.md) for wheel-building/release details.

## Requirements

- Python 3.10+
- Rust 1.75+ (only if building from source — not needed to `pip install`)
- Network access to `nominatim.openstreetmap.org` and
  `archive-api.open-meteo.com` at call time (no API key needed for either)

## License

Proprietary — see [LICENSE](LICENSE). Source is public on GitHub for
review; this isn't an open-source license, so redistribution/reuse isn't
granted by default.

## Support

- Issues: https://github.com/Mullassery/PyWeatherEnriched/issues
- Email: mullassery@gmail.com
