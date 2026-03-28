# Streaming Crypto

[![CI core](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-core.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-core.yml)
[![CI ffi](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-ffi.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-ffi.yml)
[![CI pyo3](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-pyo3.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-pyo3.yml)
[![Docs.rs](https://docs.rs/streaming-crypto/badge.svg)](https://docs.rs/streaming-crypto)
[![Crates.io](https://img.shields.io/crates/v/streaming-crypto.svg)](https://crates.io/crates/streaming-crypto)
[![PyPI](https://img.shields.io/pypi/v/streaming-crypto.svg)](https://pypi.org/project/streaming-crypto/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A Rust cryptographic library with optional FFI and Python bindings.

---

## Features

- **Rust API** (default): Pure Rust functions.
- **FFI API**: C ABI functions for integration with C/C++ and other languages.
- **PyO3 API**: Python bindings, installable via `pip`.

---

## Usage

### Rust API

```bash
cargo add streaming-crypto
```

### FFI API

```bash
cargo build --features ffi-api
```

### Python API

```bash
pip install streaming-crypto
```

---

## Documentation

- [API Docs on docs.rs](https://docs.rs/streaming-crypto)
- [Crate on crates.io](https://crates.io/crates/streaming-crypto)
- [PyPI package](https://pypi.org/project/streaming-crypto/)

---

## ✅ Badge Breakdown

- **CI badge** → shows GitHub Actions build/test status.  
- **Crates.io badge** → shows latest published version on crates.io.  
- **Docs.rs badge** → links to auto‑generated Rust documentation.  
- **PyPI badge** → shows latest published version on PyPI.  
- **License badge** → signals open‑source license clearly.  

---

## Set environment python for test

```bash
# Dynamically set DYLD_LIBRARY_PATH for Python framework
PYVER=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
export DYLD_LIBRARY_PATH="/Library/Frameworks/Python.framework/Versions/$PYVER/lib:$DYLD_LIBRARY_PATH"

# Ensure PyO3 uses the same Python interpreter
export PYO3_PYTHON=$(which python3)

cargo clean
cargo test -p streaming-crypto --no-default-features --features pyo3-api
```
