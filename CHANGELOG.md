# Changelog

All notable changes to this project are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This file was introduced 2026-09-21. Version history before that point
(`v0.1.0` through `v0.6.0`, plus the current unreleased `0.7.0` in
`Cargo.toml`) is real — see `git log` and the repo's git tags — but is
**not** being retroactively reconstructed into dated changelog entries
here, to avoid guessing at details that aren't confidently known from
the commit history alone. Note also: git tag `v2.0.0` appears to be a
tagging mistake (it points at the same commit as the "v0.2.0" release);
see `ROADMAP_HONEST.md` for detail.

## [Unreleased]

### Fixed
- `src/enhanced_cache.rs:397`: `EnhancedCache::stats()` used
  `self.stats.lock().unwrap()`, which would panic on top of a prior panic
  if the mutex were ever poisoned. Changed to
  `.unwrap_or_else(|poisoned| poisoned.into_inner())`, matching the
  established pattern in sibling repos, so a poisoned lock degrades
  gracefully instead of taking down the caller. Verified with
  `cargo test --lib` (39/39) and the `pytest` `TestEnhancedCache` suite.
- `tests/phase2_integration_test.rs` (deleted): this integration test
  imported `ParallelEnricher`, `BatchResolver`, `StreamingReader`,
  `StreamingWriter`, `DatabaseConfig`, `DatabaseType` from crate root,
  none of which are exported (the source files that define them are not
  declared as `mod`s in `src/lib.rs` — see "Orphaned source files" in
  `ROADMAP_HONEST.md`). This made plain `cargo test` (without `--lib`)
  fail to compile entirely. Deleted the stale test file only — the
  underlying orphaned modules it referenced are untouched and still need
  a real wire-up-or-delete decision. Verified `cargo test` (no `--lib`)
  now compiles and passes (39 lib tests + 0 bin/doc tests).
- `src/geospatial/reverse_geocoding.rs:289`: `processing_time_ms` in
  `CompleteReverseGeocodeResponse` was hardcoded to `0` with a `// TODO:
  Track timing`. Now measured for real with `std::time::Instant` around
  the `reverse_geocode` call in `reverse_geocode_with_detail`. Verified
  with the existing `geospatial::reverse_geocoding` unit tests (still
  39/39 passing under `cargo test --lib`). Note: `alternatives: Vec::new()`
  at line 287 (same struct) was **not** fixed — real alternatives would
  require additional geocoding sources (Google/USPS), which are
  deliberately unimplemented stubs (`src/geospatial/optional.rs`); that's
  a real feature gap, not a contained bug, and is left documented in
  `ROADMAP_HONEST.md`.
- Re-verified PyPI publish is still broken: the most recent tagged
  release run (`v0.6.0`, `gh run view 31983879736`) still fails at the
  `publish` job with `403 Invalid or non-existent authentication
  information` from `https://upload.pypi.org/legacy/`, and
  `.github/workflows/release.yml` is unchanged from the state
  `ROADMAP_HONEST.md` describes (`secrets.PYPI_API_TOKEN`, no
  `if: always()` on the GitHub Release step). No fix attempted — rotating
  the token requires repo secret access this pass doesn't have.
- `pyproject.toml`: bumped the `dev` extra's `pytest` floor from `>=7.0`
  to `>=9.0.3` — `pip-audit` found the old floor resolving to `pytest
  8.4.2`, which has a known, real vulnerability (`PYSEC-2026-1845`,
  dev/test-only dependency, no production exposure). Verified the fix
  with `pip-audit` (clean after the bump) and by re-running the full test
  suite under `pytest 9.1.1`.
- `BUILD.md`: removed stale "proprietary license" / "not open for
  redistribution" language left over from before the 2026-09-06
  Apache-2.0 relicense, and corrected the description of the release
  workflow to state plainly that the PyPI publish step is currently
  broken (stale token) rather than describing it as fully automatic.
- `.github/workflows/tests.yml`: `actions/setup-python@v4` → `v5`;
  simplified the install step (`pip install -e ".[dev]" 2>/dev/null ||
  pip install -e .` → `pip install -e ".[dev]"`) so a real install
  failure isn't silently swallowed by the `2>/dev/null` fallback.
- `.github/workflows/release.yml`: `softprops/action-gh-release@v1` →
  `v2` (old major flagged by `actionlint` as unsupported).
- `cargo fmt` formatting drift in `src/enricher.rs` and `src/geocoder.rs`
  (cosmetic only — `cargo test --lib` and `cargo clippy -- -D warnings`
  re-verified passing after).
- `README.md`: corrected the documented Rust unit test count from 32 to
  the actual, currently-passing 39.

### Added
- `.github/workflows/tests.yml`: a `rust` CI job running `cargo fmt
  --check`, `cargo clippy --lib -- -D warnings`, and `cargo test --lib`
  — previously CI only ran `pytest`, with no Rust lint/test gate at all
  despite this being a majority-Rust codebase.
- `.github/workflows/audit.yml`: a new dependency/vulnerability-scanning
  workflow (`cargo-audit` via `rustsec/audit-check`, plus `pip-audit`),
  on push/PR/weekly schedule.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `ROADMAP_HONEST.md` (new).

## Earlier history (not itemized — see git log / git tags)
`v0.1.0` → `v0.6.0`, and the current in-progress `0.7.0`.
