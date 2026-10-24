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

## 4. Not built / deliberately unimplemented

`src/geospatial/optional.rs` — `VegetationService`, `SoilService`,
`FloodRiskService`, `GoogleMapsReverseGeocoder`, `USPSPostalDatabase` are
framework stubs whose methods return a real `Err("... not yet
implemented")` rather than fabricated data (verified by reading
`optional.rs:29-31`, `:49-51`, `:68-70`, `:92-94`, `:116`). This is
honest, correct behavior for an unimplemented feature — flagged here only
for completeness, not as a problem.

---

## Technical debt (concrete, file:line)

### Orphaned/dead source files — not compiled, not tested, actively misleading
Seven files under `src/` total **1,631 lines** and are not referenced by
any `mod` declaration in `src/lib.rs`, so `cargo build`/`cargo test --lib`
never touch them at all:

| File | Lines | Referenced by |
|---|---|---|
| `src/batch_resolver.rs` | 175 | nothing |
| `src/cloud_storage.rs` | 300 | nothing |
| `src/database.rs` | 270 | nothing |
| `src/distributed_processing.rs` | 300 | nothing |
| `src/geospatial_connectors.rs` | 365 | nothing |
| `src/parallel.rs` | 79 | nothing |
| `src/streaming_io.rs` | 142 | nothing |

These were never removed after being superseded/abandoned, and a
contributor grepping `src/` would reasonably assume they're live code.
**Update (this pass)**: the stale, non-compiling `tests/
phase2_integration_test.rs` that referenced these modules has been
deleted (it broke plain `cargo test`; see "Fixed" in `CHANGELOG.md`), so
these seven files are now referenced by nothing at all, anywhere in the
repo — not even a broken test. The wire-up-or-delete decision on the
modules themselves is unchanged and still not made.
**Recommendation for a dedicated follow-up session**: either (a) actually
wire the useful ones (`parallel.rs`, `batch_resolver.rs` look closest to
reusable) into `lib.rs` with real tests, or (b) delete all seven
outright. Do not leave them as-is — they inflate the codebase's apparent
surface area by ~65% (1,631 dead lines vs. ~1,893 lines of real,
compiled, wired non-test Rust) with things reviewers/contributors may
quote as real.

### Dependency staleness — real version gaps checked against the live crates.io index (2026-09-21), not guessed
`Cargo.toml`'s version requirements act as a ceiling `cargo update` won't
cross on its own; several are now multiple majors behind current:

| Crate | `Cargo.toml` line | Locked (`Cargo.lock`) | Current latest (crates.io, checked live) | Gap |
|---|---|---|---|---|
| `pyo3` | `Cargo.toml:21` (`"0.23"`) | 0.23.5 | 0.29.2 | 6 minor releases behind; this is the core PyO3 binding layer |
| `reqwest` | `Cargo.toml:27` (`"0.11"`) | 0.11.27 | 0.13.5 | 2 majors behind |
| `rustls` (transitive, via reqwest 0.11 → hyper-rustls 0.24) | n/a (transitive) | 0.21.12 | 0.23.45 | 2 majors behind; only fixable by moving `reqwest` to 0.12+ |
| `h2` (transitive, via hyper 0.14) | n/a (transitive) | 0.3.27 | 0.4.19 | 1 major behind; only fixable via `reqwest`/`hyper` upgrade |
| `rusqlite` | `Cargo.toml:34` (`"0.31"`) | 0.31.0 | 0.40.2 | 9 minor releases behind |
| `lru` | `Cargo.toml:33` (`"0.12"`) | 0.12.5 | 0.18.4 | 6 minor releases behind |
| `redis` | `Cargo.toml:28` (`"0.25"`) | 0.25.5 | 1.7.0 | crossed a 1.0 major bump; huge gap — but see note below |
| `tiff` | `Cargo.toml:29` (`"0.9"`) | 0.9.1 | 0.11.3 | 2 minor releases behind |
| `geojson` | `Cargo.toml:30` (`"1.0"`) | 1.0.0 | 1.0.0 | up to date |
| `chrono` | `Cargo.toml:31` (`"0.4"`) | 0.4.45 | 0.4.45 | up to date |

Note on `redis`: it's only used inside the dead-code-gated
`src/geospatial/` module (`src/geospatial/data_source.rs`,
`src/geospatial/config.rs`), so the huge version gap has zero runtime
exposure today — but it will need re-verifying against 1.x's API
regardless if/when that module gets wired in.

`cargo audit` **could not be run to completion in this environment** —
the sandbox can reach `crates.io`'s sparse index (`index.crates.io`, used
above to check real version numbers) but cannot reach `github.com` over
git protocol, which `cargo-audit` needs to clone the RustSec advisory
database (`error: couldn't fetch advisory database ... github.com`).
This should work fine in real GitHub Actions (added as a CI job in
`.github/workflows/audit.yml` — see below) since GH Actions runners have
normal GitHub connectivity; it just couldn't be verified end-to-end from
here. Given the version gaps above (especially the rustls/h2 chain sitting
two majors behind on the TLS/HTTP2 stack that talks to two public HTTPS
APIs), a real `cargo audit` run against this exact `Cargo.lock` is a
priority for whoever has that CI access next.

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
2. Decide the fate of the 1,631 orphaned lines in section "Technical
   debt" above — wire in or delete, don't leave dangling.
3. Get real network access to run `cargo audit` end-to-end against this
   `Cargo.lock` (the CI job added here should do this on the next push —
   watch it).
4. Plan the `reqwest` 0.11 → 0.13 migration (and the `rustls`/`h2`
   upgrades that come free with it) as one deliberate piece of work, not
   an incidental side effect of something else — it touches
   `src/enricher.rs`, `src/geocoder.rs`, and `src/http_retry.rs`.
