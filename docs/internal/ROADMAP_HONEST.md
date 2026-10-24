# Honest Status / Roadmap

This document is deliberately blunt. No "planned"/"may be added" hedging —
if something is broken or missing, it says so plainly. Last verified
2026-09-22 by running the actual test/build/lint/audit commands listed
below on this machine (a quick-fix pass following up on the 2026-09-21
audit; see `CHANGELOG.md`'s `[Unreleased]` section for what changed).

## 1. Works, and I verified it myself

- **Forward geocoding (Nominatim) + historical weather (Open-Meteo Archive
  API)**, wired through the Rust core to Python. Verified by running the
  full test suite with a working network connection:
  `cargo test --lib` → 39/39 passed. `pytest tests/ -v` → 33/33 passed
  (includes 5 tests in `TestWeatherEnricherLive` that make real network
  calls to Nominatim + Open-Meteo).
- `cargo clippy --lib -- -D warnings` → clean, zero warnings.
- `maturin develop --release` → builds and installs successfully
  (Python 3.11, macOS ARM64, `PYO3_PYTHON` set to satisfy the
  `abi3-py310` floor).
- `maturin sdist` → the built tarball **does** include `LICENSE`
  (`pyweatherenriched-0.7.0/LICENSE` verified present in the tar
  listing), despite `[tool.maturin]` in `pyproject.toml` not listing it
  under an explicit `include`. This is the recurring
  "maturin sdist omits LICENSE" bug seen in sibling repos in this org —
  checked for here specifically, and it does **not** reproduce on this
  repo's maturin/pyproject.toml configuration.
- In-memory LRU cache (`Cache`, `src/cache.rs`) and the SQLite-backed
  `EnhancedCache` (`src/enhanced_cache.rs`) — both covered by real unit
  tests that exercise actual SQLite round-trips, TTL expiry, and
  proximity matching, not mocks.
- `enrich_range` (single-request historical backfill) — real, tested
  (`enricher::tests::test_enrich_range_fetches_whole_range_in_one_request`).
- HTTP retry/backoff (`src/http_retry.rs`) — real, tested against 429s,
  5xxs, and `Retry-After` handling with `mockito`.

## 2. Works but is intentionally NOT exposed / NOT wired in

- `src/geospatial/` (elevation/lapse-rate via real SRTM GeoTIFF parsing,
  urban-heat-island via real OSM building-density analysis, OSM reverse
  geocoding) — real logic, independently unit-tested
  (`cargo test --lib` runs and passes these), but the whole module is
  `#[allow(dead_code)]` in `src/lib.rs:15` and not reachable from the
  `#[pymodule]` — no Python user can call any of it today. Needs a
  decision: wire it into the Python API with an end-to-end test, or
  delete it. Neither has happened; it just sits there.
- `EnhancedCache` is not automatically used by `WeatherEnricher.enrich()`
  — it's a separate opt-in class. Small, known, real gap.

## 3. Broken

- **PyPI publish is broken.** `.github/workflows/release.yml`'s `publish`
  job uploads to PyPI using `secrets.PYPI_API_TOKEN`; the token is
  stale/invalid and PyPI returns `403 Invalid or non-existent
  authentication information`. Because the "Create GitHub Release" step
  has no `if: always()` / `continue-on-error`, a failed PyPI upload also
  means **no GitHub Release is ever created**, even though Linux/macOS/
  Windows wheels were built successfully as artifacts. Net effect: only
  one wheel (macOS ARM64, uploaded manually via `twine` at some point)
  is live on PyPI; everyone else installing on Linux/Windows/x86 macOS
  gets a from-source sdist build instead of the wheel CI already built
  for them. Fix requires rotating/re-issuing the PyPI token as a repo
  secret — cannot be done from within this environment.
- **`git tag v2.0.0` looks like a tagging mistake.** It points at the same
  commit as the "v0.2.0 release" commit (`feabbe2`, 2026-07-30) — almost
  certainly meant to be `v0.2.0`, not `v2.0.0`. Left as-is (not deleting
  a pushed tag without being asked), but anyone consuming tags/releases
  should be aware `v2.0.0` does not mean what it looks like it means.
- ~~**`cargo test` (without `--lib`) does not compile.**~~ **Fixed**:
  `tests/phase2_integration_test.rs` (which imported `ParallelEnricher`,
  `BatchResolver`, `StreamingReader`, `StreamingWriter`,
  `DatabaseConfig`, `DatabaseType` from `pyweatherenriched` — none of
  which were ever exported, since the source files that define them are
  not declared as `mod`s in `src/lib.rs`) has been deleted. Plain
  `cargo test` now compiles and passes. The underlying orphaned modules
  (`src/parallel.rs`, `src/batch_resolver.rs`, `src/streaming_io.rs`,
  `src/database.rs`, etc.) themselves are untouched — see "Orphaned
  source files" below, still needs a real wire-up-or-delete decision.

## 4. Optional geospatial layers

`VegetationLayer`, `SoilLayer`, and `FloodRiskLayer` are now real,
GeoJSON-tile-backed services exposed to Python. They return only properties
present in the tile and error on missing tiles/properties. Google Maps and
USPS reverse geocoding remain deliberately unimplemented (use the
OSM/Nominatim path).

---

## Technical debt (concrete, file:line)

### Orphaned/dead source files — resolved
The seven uncompiled, unreferenced files previously listed here
(`batch_resolver.rs`, `cloud_storage.rs`, `database.rs`,
`distributed_processing.rs`, `geospatial_connectors.rs`, `parallel.rs`,
`streaming_io.rs`; 1,631 lines) have been deleted. They contained mock/
placeholder implementations and were not wired into `lib.rs` or Python, so
they appeared in the source tree without being reachable or tested. Git
history retains them if any logic is worth revisiting.

### Dependency staleness — resolved for the HTTP/TLS stack
`pyo3` is now 0.29, `reqwest` is 0.12, and the transitive `rustls`/`h2`
stack is current. `cargo audit` exits clean against the committed
`Cargo.lock`; CI runs it. Remaining version gaps (`rusqlite`, `lru`,
`tiff`, the dead-code-gated `redis` 0.25) are non-security and can be
handled separately.

`pip-audit` **was** run successfully (network to PyPI worked):
```
pip-audit -r <(printf 'pandas>=1.5\nnumpy>=1.23\npytest>=7.0\n')
```
found **one real, confirmed vulnerability**: `pytest 8.4.2` (resolved
from the old `pytest>=7.0` floor) → `PYSEC-2026-1845`, fixed in
`9.0.3`. This is a dev/test-only dependency (no production/runtime
exposure), but it was real. **Fixed in this pass**: `pyproject.toml`'s
`dev` extra now requires `pytest>=9.0.3`; re-ran `pip-audit` against the
new floor and it reports "No known vulnerabilities found", and the full
`pytest` suite was re-verified passing under the installed `pytest
9.1.1`.

### Minor code-quality items
- ~~`src/enhanced_cache.rs:397` — `self.stats.lock().unwrap()`~~ **Fixed**:
  changed to `.unwrap_or_else(|poisoned| poisoned.into_inner())` so a
  poisoned mutex degrades gracefully instead of panicking again. Verified
  with `cargo test --lib` and `pytest`.
- `src/geospatial/reverse_geocoding.rs:287` — `alternatives: Vec::new()`
  with a `// TODO: Get alternatives from multiple sources` — still not
  fixed. Real alternatives would require additional geocoding sources
  (Google/USPS), which are deliberately unimplemented stubs
  (`src/geospatial/optional.rs`) — this is a real feature gap, not a
  contained bug, so it's left as-is.
- ~~`src/geospatial/reverse_geocoding.rs:289` — `processing_time_ms: 0`~~
  **Fixed**: now measured for real with `std::time::Instant` around the
  `reverse_geocode` call in `reverse_geocode_with_detail`. Verified with
  the existing `geospatial::reverse_geocoding` unit tests.
- Both of the above are inside the dead-code-gated `geospatial` module
  (see section 2), so neither has user-facing impact today, but the
  remaining `alternatives` gap would need addressing before that module
  is ever wired up for real.

### CI/process gaps found and fixed in this pass
- `.github/workflows/tests.yml` ran **only** `pytest` — there was no CI
  job at all for `cargo fmt`, `cargo clippy`, or `cargo test`, despite
  this being a majority-Rust codebase. **Fixed**: added a `rust` job
  running `cargo fmt --check`, `cargo clippy --lib -- -D warnings`, and
  `cargo test --lib`.
- No dependency/vulnerability-scanning CI job existed at all. **Fixed**:
  added `.github/workflows/audit.yml` running `cargo-audit` (via
  `rustsec/audit-check`) and `pip-audit`, on push/PR/weekly schedule.
  This could not be validated end-to-end from this sandboxed environment
  (see `cargo audit` network note above) — verify it actually goes green
  on the next real push.
- `tests.yml`'s install step was `pip install -e ".[dev]" 2>/dev/null ||
  pip install -e .` — the `2>/dev/null` silently swallowed the real error
  output of the primary install command before falling back, which would
  have made a real install failure hard to diagnose from CI logs. Since
  the `dev` extra is always defined in this repo's `pyproject.toml`, the
  fallback branch was realistically unreachable dead code anyway.
  **Fixed**: simplified to a single `pip install -e ".[dev]"`.
- `actionlint` flagged two outdated action versions, both fixed:
  `actions/setup-python@v4` → `v5` (`tests.yml`), and
  `softprops/action-gh-release@v1` → `v2` (`release.yml`).
- `cargo fmt --check` had drifted (8 diff hunks across
  `src/enricher.rs` and `src/geocoder.rs`, both pre-existing, plus the
  already-dead `tests/phase2_integration_test.rs`). **Fixed** by running
  `cargo fmt`; purely cosmetic, no behavior change (`cargo test --lib`
  and `cargo clippy` re-verified passing after).
- `BUILD.md` still described the license as "proprietary ... not open for
  redistribution/reuse without permission" and PyPI publishing as fully
  automatic — both stale since the 2026-09-06 Apache-2.0 relicense and
  since the PyPI token broke. **Fixed**: rewrote the affected sections
  to match `LICENSE`/`README.md`/CI reality.

---

## What would make a real difference next (not started, needs a dedicated session)

1. Rotate the PyPI API token and get the `release.yml` publish step
   actually green again — right now most platforms silently don't get a
   wheel.
2. ~~Decide the fate of the 1,631 orphaned lines~~ **Done:** the dead
   modules were deleted; see the technical-debt section above.
3. ~~Run `cargo audit` end-to-end~~ **Done:** it is clean and enforced in CI.
4. ~~Plan the `reqwest` 0.11 → 0.13 migration~~ **Done:** `reqwest` is on
   0.12 and the `rustls`/`h2` stack is current.
