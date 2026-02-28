# streaming-crypto

[![CI](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/streaming-crypto.svg)](https://crates.io/crates/streaming-crypto)
[![Docs.rs](https://docs.rs/streaming-crypto/badge.svg)](https://docs.rs/streaming-crypto)
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

1. **Before:** PyO3 was trying to link against Python 3.13 because either a cached build or environment variable made it think our Python was 3.13.

2. **Action:** We forced PyO3 to use Python 3.12 or current version of `pyenv` explicitly:

    ```bash
    export PYTHON_SYS_EXECUTABLE="$(pyenv which python3)"
    export PYO3_PYTHON="$(pyenv which python3)"
    export PYO3_NO_PYTHON_LINK=1
    cargo clean
    ```

3. **Result:** Cargo rebuilds PyO3 and all dependent crates from scratch. Now `cargo run --bin check_python` correctly detects Python 3.12.12 and does **not crash on missing libpython3.13.dylib**.

4. **cargo test** will work now
