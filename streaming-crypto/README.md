# streaming-crypto

[![CI](https://github.com/username/streaming-crypto/actions/workflows/ci.yml/badge.svg)](https://github.com/username/streaming-crypto/actions/workflows/ci.yml)
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

---

### 1. Repository setup

We need to **create the GitHub repository in advance** on GitHub.com.  

- Push our initial codebase there.  
- In the repo settings, add our **secrets** (`CARGO_REGISTRY_TOKEN`, `PYPI_API_TOKEN`) under **Settings → Secrets and variables → Actions**.  
- This ensures our workflow has the environment it needs when we eventually tag a release.

---

### 2. Controlling publish triggers

Our workflow is configured to run on:

```yaml
on:
  push:
    tags:
      - 'v*.*.*'
```

That means **only pushes with a tag like `v0.1.0` or `v1.2.3` will trigger publish**.  

- If we just push commits to `main` (without tags), the publish job won’t run.  
- Our CI workflow (build/test) runs on `push` and `pull_request` to `main`, but publish is tag‑only.

---

### 3. Recommended flow

- **Step 1:** Create the repo on GitHub.  
- **Step 2:** Push our initial codebase (no tags yet). This will run CI (build/test) but not publish.  
- **Step 3:** Configure secrets (`CARGO_REGISTRY_TOKEN`, `PYPI_API_TOKEN`).  
- **Step 4:** Once CI is green and we’re ready for a release, create a version tag locally:  

```bash
git tag v0.1.0
git push origin v0.1.0
```

- **Step 5:** That tag push will trigger the publish workflow, which will publish to crates.io and PyPI.

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
