# Audit Findings: PyWeatherEnriched

**Audit Date:** 2026-10-26  
**Version:** 0.7.2  
**Branch:** `master`  
**Full report:** [`.audit/reports/PyWeatherEnriched/audit.md`](../.audit/reports/PyWeatherEnriched/audit.md)

## Scope

Read-only review of the enrich/geocode/cache core, PyO3 bindings, Python feature layer, geospatial modules, tests, CI, and release documentation. No live external APIs were called.

## Findings

| ID | Severity | Finding | Evidence | Recommendation |
|---|---|---|---|---|
| PWE-01 | Medium | Blocking network/SQLite work runs while holding the Python GIL. | `src/python_bindings.rs`, `src/geospatial/python_api.rs` lack `detach`/`allow_threads`. | Release the GIL or provide async/concurrent batch APIs. |
| PWE-02 | Medium | Nominatim has no explicit rate limiter. | `src/geocoder.rs` retries but never enforces 1 req/s. | Add a serialized global rate limiter. |
| PWE-03 | Medium | SQLite proximity lookup is not nearest-neighbor and can return the wrong/missing row. | `find_nearby_in_db`: fixed ±0.05° box, `LIMIT 1`, no `ORDER BY`, post-filter. | Use a radius-aware box and distance ordering; return matched coordinates. |
| PWE-04 | Low/Medium | Proximity hits return the cached location, not the requested one. | `find_nearby_cached` returns cached `EnrichedData`. | Make substitution explicit or disable it by default. |
| PWE-05 | Low/Medium | Primary cache is a capacity-limited `HashMap`, not LRU. | `src/enricher.rs`; README claims LRU. | Use a real LRU or correct documentation. |
| PWE-06 | Low | Geocoder cache is unbounded. | `Mutex<HashMap<String, (f64, f64)>>`. | Add size/TTL bounds. |
| PWE-07 | Low | Batch failures lose their cause. | `enrich_batch` emits `"Enrichment failed"`. | Return error type/message or fail-fast option. |
| PWE-08 | Low/Medium | Version/test/cargo-test documentation is stale. | README 0.7.0/39 tests/stale `cargo test` warning; Cargo 0.7.2; 50 Rust tests; stale integration test deleted. | Unify release facts at 0.7.2. |
| PWE-09 | Low/Medium | No concurrency, load, or proximity edge-case tests. | CI is unit-test only. | Add 1k/10k batch, threaded, and radius/box-corner tests. |
| PWE-10 | Low | SQLite cache has no encryption or ACL. | Caller-supplied `db_path`. | Document retention/permissions and consider optional encryption. |

## Positive Controls

- Real Nominatim/Open-Meteo integration; no fabricated data.
- Descriptive Nominatim User-Agent and 10s timeout.
- Bounded retries and explicit HTTP/parse errors.
- Enhanced cache handles poisoned locks gracefully.
- Good Rust coverage of parsing, retries, geospatial fixtures, and bindings.
- Honest documentation of unimplemented optional layers and broken PyPI publish.
- Network-gated live tests avoid breaking offline CI.

## Audit Limitations

- No live API calls; quota/rate behavior was not observed.
- Proximity issues were found by static inspection, not a new integration test.
- No GIL-contention benchmark.
- Test counts are definitions, not a verified pass count.
- Dependency advisories were not independently re-scanned.
