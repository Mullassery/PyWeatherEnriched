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
