# Build Instructions

PyWeatherEnriched is distributed as **wheels only**. Source code is proprietary and not included in the distribution.

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

The repository includes a GitHub Actions workflow (`.github/workflows/release.yml`) that:
- Builds wheels for Linux, macOS, and Windows
- Runs on each release tag
- Automatically uploads to PyPI

To create a release:
```bash
git tag v0.1.0
git push origin v0.1.0
```

## Wheel Contents

Each wheel contains:
- `pyweatherenriched` compiled extension module (.so/.pyd)
- Python stubs for type checking
- Metadata and license information

**NO SOURCE CODE** is included in wheels (proprietary).

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
```bash
sudo apt-get install libssl-dev
```

## Distribution Policy

- **PyPI**: Wheels only (verified via `pip install`)
- **GitHub Releases**: Wheels as binary artifacts
- **Source Code**: Proprietary, not distributed
- **License**: Proprietary license (see LICENSE file)

## Support

For build issues:
- GitHub Issues: https://github.com/Mullassery/PyWeatherEnriched/issues
- Email: mullassery@gmail.com

## Development Setup (For Contributors with Source Access)

If you have access to the source code:

```bash
# Clone repository
git clone https://github.com/Mullassery/PyWeatherEnriched.git
cd PyWeatherEnriched

# Build development mode
maturin develop

# Run tests
cargo test

# Build optimized wheels
maturin build --release
```

**Note**: Source code access is restricted and only available to authorized contributors under the proprietary license.
