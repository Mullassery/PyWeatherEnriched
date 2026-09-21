# Contributing

Thanks for considering a contribution to PyWeatherEnriched.

## Development setup

```bash
git clone https://github.com/Mullassery/PyWeatherEnriched.git
cd PyWeatherEnriched

python3 -m venv .venv && source .venv/bin/activate
pip install maturin
maturin develop --release   # builds the Rust extension, installs it editable

pip install -e ".[dev]"
```

Requires Python 3.10+ and Rust 1.75+. `pyo3`'s `abi3-py310` feature means
the Python interpreter you build with must be 3.10 or newer even though
the resulting wheel is ABI-stable across later 3.x versions.

## Running the test suite before you open a PR

```bash
cargo fmt --check                    # formatting
cargo clippy --lib -- -D warnings    # lints, warnings are errors
cargo test --lib                     # Rust unit tests (39 tests)
pytest tests/ -v                     # Python tests (33 tests)
```

All four of these are also what CI runs (`.github/workflows/tests.yml`) —
if they pass locally they should pass in CI.

Note: plain `cargo test` (without `--lib`) does not compile —
`tests/phase2_integration_test.rs` is a stale integration test referencing
modules that were never wired into `src/lib.rs`. See
[ROADMAP_HONEST.md](ROADMAP_HONEST.md) for the full story; use
`cargo test --lib` until that's resolved.

Some Python tests (`TestWeatherEnricherLive` in
`tests/test_python_bindings.py`) make real network calls to Nominatim and
Open-Meteo. They need outbound network access and will fail (not
necessarily indicating a code problem) if either service is unreachable
or rate-limits you.

## Code style

- Rust: standard `rustfmt` defaults, `clippy` clean with `-D warnings`.
- Python: no formatter/linter is currently enforced in CI for the
  `python/` package or `tests/*.py` — if you add one (e.g. `ruff`,
  `black`), wire it into `.github/workflows/tests.yml` in the same PR so
  it's actually enforced, not just recommended.
- Comments that explain *why*, not *what*, are preferred, especially
  around anything non-obvious (see the existing comments in
  `Cargo.toml` and `src/lib.rs` for the house style).

## Honesty about status

This project's docs (`README.md`, `ROADMAP_HONEST.md`) are intentionally
blunt about what's real, tested, broken, or unbuilt. If you add a
feature, please describe it the same way: state plainly whether you
tested it and how, rather than describing untested code as working. If
you're adding a stub for something not yet implemented, make it fail
loudly with a clear error (see `src/geospatial/optional.rs` for the
existing pattern) rather than returning fabricated/placeholder data.

## Reporting bugs / requesting features

Use the GitHub issue templates
(`.github/ISSUE_TEMPLATE/`). For security issues, see
[SECURITY.md](SECURITY.md) instead of opening a public issue.

## Pull requests

- Keep PRs focused — one logical change per PR.
- Update `README.md`/`ROADMAP_HONEST.md`/`CHANGELOG.md` if your change
  affects what they claim about the project's status.
- Add or update tests for behavior you change.
