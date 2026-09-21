# Build Instructions

PyWeatherEnriched is source-available on GitHub at
https://github.com/Mullassery/PyWeatherEnriched under the Apache License 2.0
(see `LICENSE`). It is intended to ship to PyPI as prebuilt wheels, but see
the "Distribution Policy" note below — the CI→PyPI publish step is
currently broken, so only a macOS ARM64 wheel is actually published today.

## Installation from PyPI

```bash
pip install pyweatherenriched
```

Requires Python 3.10 or higher.

Supported platforms:
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

## Building Wheels (For Maintainers)

PyWeatherEnriched uses Maturin to build PyO3 wheels from Rust source code.

### Prerequisites

1. **Rust toolchain** (latest stable):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. **Python 3.10+** with development headers:
   ```bash
   # Linux (Debian/Ubuntu)
   sudo apt-get install python3.10-dev

   # macOS
   brew install python@3.10

   # Windows
   # Install from python.org or use Windows Store
   ```

3. **Maturin**:
   ```bash
   pip install maturin
   ```

### Build Steps

1. **Build wheels for your current platform**:
   ```bash
   cd /path/to/PyWeatherEnriched
   maturin build --release
   ```

   Wheels appear in `target/wheels/`

2. **Build wheels for multiple Python versions**:
   ```bash
   maturin build --release -i python3.10 -i python3.11 -i python3.12 -i python3.13
   ```

3. **Build wheels for multiple platforms** (cross-compilation):
   ```bash
   # Linux x86_64 and ARM64
   maturin build --release --target x86_64-unknown-linux-gnu
   maturin build --release --target aarch64-unknown-linux-gnu

   # macOS Intel and Apple Silicon
   maturin build --release --target x86_64-apple-darwin
   maturin build --release --target aarch64-apple-darwin

   # Windows x86_64
   maturin build --release --target x86_64-pc-windows-msvc
   ```

### Upload to PyPI

Using Twine:

```bash
# Install twine
pip install twine

# Upload wheels
twine upload target/wheels/pyweatherenriched*.whl

# With token authentication
twine upload --repository pypi target/wheels/pyweatherenriched*.whl
```

## GitHub Actions (Automatic Builds)

The repository includes a GitHub Actions workflow (`.github/workflows/release.yml`) that,
on each `v*` tag push:
- Builds wheels for Linux (x86_64/ARM64), macOS (Intel/Apple Silicon), and Windows (x86_64)
- Attempts to upload them to PyPI

**This publish step is currently broken**: PyPI rejects the upload with
`403 Invalid or non-existent authentication information` (a stale/invalid
`PYPI_API_TOKEN` secret), so only a macOS ARM64 wheel — uploaded manually
via `twine` at some point in the past — is actually live on PyPI today. The
built wheels for every other platform exist only as CI build artifacts on
a given workflow run, not on PyPI, until someone fixes the token and/or
runs a manual `twine upload` (see the Manual Upload section above). See
[ROADMAP_HONEST.md](ROADMAP_HONEST.md) for tracking.

To create a release:
```bash
git tag v0.1.0
git push origin v0.1.0
```

## Wheel Contents

Each wheel contains:
- `pyweatherenriched._pyweatherenriched`, the compiled Rust/PyO3 extension module (.so/.pyd)
- The pure-Python `pyweatherenriched` package (`__init__.py`, `features.py`)
- Metadata and license information

## Troubleshooting

### "No module named 'maturin'"
```bash
pip install maturin
```

### Python version mismatch
Ensure your Python installation matches the wheel being built:
```bash
python --version
maturin build -i /path/to/python3.10
```

### Rust compilation errors
Update Rust:
```bash
rustup update stable
```

### OpenSSL errors on Linux
Shouldn't happen: `reqwest` is configured with `rustls-tls` (pure Rust TLS,
no system OpenSSL) specifically so this class of error doesn't come up —
see the comment on the `reqwest` dependency in `Cargo.toml`. If you do hit
an OpenSSL-related error, it's most likely coming from a transitive
dependency other than `reqwest`; report it rather than assuming
`libssl-dev` is the fix.

## Distribution Policy

- **PyPI**: Only a macOS ARM64 wheel is currently live (`pip install` on
  any other platform falls back to a source build). See the "GitHub
  Actions" section above for why.
- **GitHub Releases**: Wheels for all platforms are built by CI on each
  tag but are currently only reachable as workflow-run artifacts, not
  attached to a GitHub Release, since the `publish` job's PyPI step fails
  before the release-creation step runs.
- **Source**: Public on GitHub under the Apache License 2.0 (see `LICENSE`) — free to use, modify, and redistribute under that license's terms.

## Support

For build issues:
- GitHub Issues: https://github.com/Mullassery/PyWeatherEnriched/issues
- Email: mullassery@gmail.com

## Development Setup

```bash
# Clone repository
git clone https://github.com/Mullassery/PyWeatherEnriched.git
cd PyWeatherEnriched

# Build development mode (installs the extension into your active venv)
maturin develop --release

# Run the Rust test suite (plain `cargo test`, without --lib, currently
# fails to compile — see the "Development" section of README.md for why)
cargo test --lib

# Run the Python test suite (needs `maturin develop` first)
pip install -e ".[dev]"
pytest tests/ -v

# Build optimized wheels
maturin build --release
```
