# Competitive intelligence in roadmap and upgrade work

**Policy date:** 2026-10-19

Competitive findings are an input to product decisions, not a one-off report.
Every pending roadmap item, new feature, and routine dependency upgrade should
check the repository's [`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md) before work
starts, and record the decision in the PR or changelog.

## Required check before building

1. **Read `AUDIT_FINDINGS.md`.** Find the relevant competitor row and the
   ranked next actions. If the finding is stale, update it with a dated source.
2. **Name the competitor and the user problem.** A feature is not justified by
   "competitor has it". State the workflow it unlocks, who needs it, and how
   often. Mark unverified demand as a hypothesis.
3. **Prefer reuse over new code.** Check whether an existing Mullassery repo
   already provides the capability (see `BUNDLES.md` in the profile repo and
   the bundle role file in this repo, when present). Typical reuse:
   - crawling/fetching/robots/SSRF → **MudFish**
   - architecture maps / PR diffs → **CodeAtlas**
   - data-quality contracts / validation → **StatGuardian**
   - token counting / cost tables → **PyTokenCalc**
   - dependency graph / advisories / SBOM → **PyDeepMesh**
   - segmentation / CLV / churn → **ClusterAudienceKit**
   - activation / lineage / PII → **PyReverseETL**
   - fleet state sync → **SyncFleet** (only after conflict resolution)
4. **Choose one disposition:** build, integrate, defer, or reject. Write it
   down. A rejection with a reason is a successful outcome.
5. **For routine upgrades**, check whether the upgrade closes a known
   competitor gap or breaks a differentiated capability. Run the tests that
   cover the affected path; do not merge a green build that silently changes a
   user-visible contract.

## PR / changelog checklist

- [ ] Competitor and source URL/date recorded (or "no direct competitor").
- [ ] User impact and frequency stated; unverified demand labelled.
- [ ] Reuse check done against sibling repos/bundles.
- [ ] Failure mode and rollback considered (especially auth, PII, cost).
- [ ] Docs/roadmap status updated honestly.
- [ ] Tests or a manual verification command included.

## Review cadence

- Refresh the competitor row when a release changes the capability.
- Re-check the full `AUDIT_FINDINGS.md` at least quarterly and before a major
  release.
- If a finding is wrong or stale, correct it in the same PR that relies on it.

## Non-goals

- Copying features for parity.
- Adding a dependency just because a competitor uses it.
- Treating star counts or download counts as proof of user need.

## OSS gap analysis — 2026-10-24

**What matters most here:** gap handling and geocoder fallback. Omitting missing hours breaks ML windows, and a single Nominatim dependency has no SLA; add an interpolation policy and provider fallback.

Scope: PyWeatherEnriched 0.7.2 (Apache-2.0). Rust/PyO3 core that forward-geocodes place names via Nominatim, fetches real historical hourly weather from the Open-Meteo Archive API, and adds pandas/numpy climate features (degree-days, cyclical time, rolling aggregates, anomalies, normals, heat index, GDD). Two-tier LRU + SQLite cache, local OSM reverse geocoding, UHI and lapse-rate helpers, local OSM/SRTM tile processing. Compared against the open-source data sources and libraries a user would assemble instead. Evidence: `README.md`, `AUDIT_FINDINGS.md`, `ROADMAP_HONEST.md`, `CHANGELOG.md`, `Cargo.toml`, `pyproject.toml`, `python/pyweatherenriched/__init__.py`, `src/lib.rs`, and verified upstream pages. README (newer) and source are authoritative where ROADMAP is stale about exposure.

### Comparable OSS projects
| Project | GitHub | License | Last activity | Why it is the comparison |
| --- | --- | --- | --- | --- |
| Open-Meteo | https://github.com/open-meteo/open-meteo | AGPL-3.0 for code; data CC BY 4.0 | release `v1.7.0`, 2026-10-16 | The actual data source; 80-year Historical Weather API, Geocoding API, self-hostable, no API key |
| Meteostat | https://github.com/meteostat/meteostat-python | MIT for code; data CC BY 4.0 | release `2.1.5`, 2026-09-17 | The closest Python historical-weather library: station lookup, daily/hourly series, and point interpolation |
| geopy | https://github.com/geopy/geopy | MIT | release `2.5.0`, 2026-07-12 | The standard Python geocoding client with Nominatim, Google and many other providers |
| timezonefinder | https://github.com/jannikmi/timezonefinder | MIT | release `9.2.0`, 2026-10-20 | Offline lat/lon to timezone lookup that weather-feature pipelines typically add for local-time correctness |

### Capability comparison
| Capability | Ours | Meteostat | Open-Meteo | geopy | Gap |
| --- | --- | --- | --- | --- | --- |
| Historical hourly observations | Yes | Yes | Yes | No | All weather options serve real observations; Ours and Open-Meteo use the same archive |
| Place-name forward geocoding | Yes | No | Yes | Yes | Ours bundles it; Open-Meteo exposes a separate Geocoding API; Meteostat requires coordinates |
| Multi-provider geocoding / fallback | No | No | Partial | Yes | Ours is Nominatim-only; geopy supports many providers |
| Date-range backfill in one call | Yes | Yes | Yes | No | Ours `enrich_range` matches the upstream single-request pattern |
| Station interpolation / gap filling | No | Yes | Partial | No | Ours omits missing hours instead of interpolating; Meteostat interpolates nearby stations |
| Built-in climate feature engineering | Yes | No | No | No | Degree-days, cyclical encodings, rolling stats, anomalies, normals are Ours only |
| Persistent/proximity cache | Yes | No | No | No | SQLite TTL + 10km proximity matching and batch dedupe are Ours only |
| Local-time / timezone normalisation | Partial | No | Yes | No | Ours accepts timestamps but does not resolve place timezone; Open-Meteo has a `timezone` parameter |
| Bulk offline operation | Partial | Partial | Yes | No | Ours caches only previously fetched keys; Open-Meteo can be self-hosted, Meteostat ships data dumps |
| Permissive licence for commercial use | Yes | Yes | Partial | Yes | Ours/geopy/Meteostat are permissive; Open-Meteo code is AGPL and hosted free tier is non-commercial |

### Gaps that matter (ranked)
1. **No interpolation or gap policy (weaker).** `README.md` says hours with no observation are omitted rather than filled, and there is no station-neighbour interpolation. Meteostat's `ms.interpolate(ts, POINT)` fills from nearby stations. Hurts ML users whose feature windows break on gaps. Effort: S-M.
2. **Single geocoder with no fallback (absent).** Nominatim is the only forward geocoder; there is no Google/Bing/Mapbox option or retry across providers, unlike geopy. Public Nominatim has usage policy limits and no SLA. Hurts production reliability and batch throughput. Effort: S.
3. **No timezone/local-time resolution (weaker).** The package has no timezonefinder-style lookup or per-location local-time columns; `add_cyclical_time_features` uses whatever timestamp is supplied. Weather features such as hour-of-day and degree-day boundaries are wrong if the input is UTC. Hurts correctness for any multi-region dataset. Effort: S.
4. **Bulk/offline workflows are thin (absent/weaker).** `EnhancedCache` is a per-key cache, not a bulk data store; there is no station-dump import or self-hosted backend. Open-Meteo can be self-hosted and Meteostat ships bulk data. Hurts reproducibility and large backfills, and the README calls out no offline mode. Effort: M-L.
5. **Distribution is broken (absent operationally).** `ROADMAP_HONEST.md` and `README.md` state that PyPI publishing fails with a 403 from a stale token, so only a macOS ARM64 wheel plus sdist is live; Linux/Windows users build from source. Hurts adoption more than any feature gap. Effort: S, but needs repository secret access.
6. **Value over raw Open-Meteo is not benchmarked beyond one micro-test (weaker).** The README reports 2.63s vs 1.26s for 3 locations and 11 engineered features; there is no accuracy or scale comparison. Users can call the API directly. Hurts the case for adding a dependency. Effort: S.

### Where we are differentiated
- **One-call enrichment + features:** geocode, fetch, and produce degree-days, cyclical encodings, rolling stats, anomalies, normals, heat index and GDD; none of the three comparables does the feature layer.
- **Cache design:** in-memory LRU plus optional SQLite with TTL, geospatial proximity hits, batch deduplication and date-range queries.
- **Honest failure semantics:** failed rows get `NaN` and missing hours are omitted; no fabricated weather, and unimplemented layers return explicit errors.
- **Rust core with Python ergonomics:** PyO3/abi3 extension with pandas bridge, small memory footprint, and a validated retry/backoff HTTP path.
- **Local geospatial extras:** OSM reverse geocoding, building-footprint UHI estimate, lapse-rate and terrain-roughness helpers without another hosted API.

### Disposition
- **Build:** optional interpolation with an explicit `interpolated` flag; timezone/local-time columns; geocoder fallback interface; bulk preload/import for cache; a reproducible accuracy/scale benchmark vs raw Open-Meteo and Meteostat.
- **Integrate/reuse:** timezonefinder for offline timezone lookup; geopy or a small provider adapter for alternate geocoders; Open-Meteo self-hosting for offline/air-gapped deployments; reuse sibling StatGuardian for output validation if needed.
- **Defer/reject:** forecasts and real-time weather (reject, out of scope); commercial reverse-geocoding backends, NDVI/soil/flood layers (defer until a user validates them); station-network interpolation at Meteostat's scale (defer, integrate if demand appears).

### Sources
- https://github.com/open-meteo/open-meteo (accessed 2026-10-24) — AGPL-3.0, release `v1.7.0` (2026-10-16), 80-year historical API, Geocoding API, self-hosting
- https://github.com/meteostat/meteostat-python (accessed 2026-10-24) — MIT, release `2.1.5` (2026-09-17), station lookup and `interpolate`
- https://github.com/geopy/geopy (accessed 2026-10-24) — MIT, release `2.5.0` (2026-07-12), multiple geocoder providers
- https://github.com/jannikmi/timezonefinder (accessed 2026-10-24) — MIT, release `9.2.0` (2026-10-20), offline timezone lookup
- Repo files inspected: `README.md`, `AUDIT_FINDINGS.md`, `ROADMAP_HONEST.md`, `CHANGELOG.md`, `Cargo.toml`, `pyproject.toml`, `python/pyweatherenriched/__init__.py`, `python/pyweatherenriched/features.py`, `src/lib.rs`
