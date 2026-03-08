# Streaming Crypto

[![CI core](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-core.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-core.yml)
[![CI ffi](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-ffi.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-ffi.yml)
[![CI pyo3](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-pyo3.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci-pyo3.yml)
[![Docs.rs](https://docs.rs/streaming-crypto/badge.svg)](https://docs.rs/streaming-crypto)
[![Crates.io](https://img.shields.io/crates/v/streaming-crypto.svg)](https://crates.io/crates/streaming-crypto)
[![PyPI](https://img.shields.io/pypi/v/streaming-crypto.svg)](https://pypi.org/project/streaming-crypto/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

With three sub-projects (`core-api`, `ffi-api`, `pyo3-api`) we’ll have a clean separation of concerns:

- **`core-api`** → Pure Rust library, where we run tests/benchmarks without FFI overhead.  
- **`ffi-api`** → Generic C ABI bindings (`extern "C"`), usable from C/C++ or other languages.  
- **`pyo3-api`** → Python‑specific bindings using PyO3 (`#[pyfunction]`, `#[pymodule]`).  
- **`streaming-crypto`** → The publishable crate that unifies everything with optional features.

---

## 📂 Project Structure

```bash
streaming-crypto/
├── Cargo.toml            # Workspace manifest (not published)
├── core-api/
│   ├── Cargo.toml        # Pure Rust library
│   └── src/lib.rs
├── ffi-api/
│   ├── Cargo.toml        # FFI bindings crate
│   └── src/lib.rs
├── pyo3-api/
│   ├── Cargo.toml        # PyO3 bindings crate
│   └── src/lib.rs
└── streaming-crypto/
    ├── Cargo.toml        # Publishable crate manifest
    ├── README.md
    ├── LICENSE
    ├── .gitignore
    └── src/
        ├── lib.rs        # Core Rust API, FFI bindings, PyO3 bindings
```

---

## 📝 Root Workspace Cargo.toml

```bash
[workspace]
members = [
    "core-api",
    "ffi-api",
    "pyo3-api",
    "streaming-crypto"
]

resolver = "2"
```

---

## 📝 streaming-crypto/Cargo.toml (Publishable Crate)

```bash
[package]
name        = "streaming-crypto"
# GITHUB TAG: v0.1.0-alpha.18-crates.0, v0.1.0-alpha.18-pypi.0
version     = "0.1.0-alpha.18"
edition     = "2021"
license     = "MIT"
description = "Cryptographic library with optional FFI and Python bindings"
repository  = "https://github.com/DreamzIt02/streaming-crypto"
keywords    = ["crypto", "ffi", "rust", "python", "bindings"]
categories  = ["cryptography", "api-bindings"]

[features] 
# --- FEATURES DYNAMIC START ---
# This block will be replaced with Publish Block, content from Cargo.publish.features
# Any common features for development and publish must be added at the end of this block
default  = ["core-api"]
core-api = ["dep:core-api"]
ffi-api  = ["dep:ffi-api"]
pyo3-api = ["dep:pyo3-api", "pyo3"]
# --- FEATURES DYNAMIC END ---

[dependencies]
# --- DEPENDENCIES DYNAMIC START ---
# This block will be replaced with Publish Block, content from Cargo.publish.dependencies
# Any common dependencies for development and publish must be added at the end of this block
core-api = { path = "../core-api", optional = true }
ffi-api  = { path = "../ffi-api",  optional = true }
pyo3-api = { path = "../pyo3-api", optional = true }

pyo3     = { version = "0.22", features = ["auto-initialize"], optional = true }
# --- DEPENDENCIES DYNAMIC END ---

[lib]
name       = "streaming_crypto"
crate-type = ["rlib", "cdylib"]
```

---

## 🚀 Usage

- **Rust API (default)**:

```bash
cargo add streaming-crypto
```

- **FFI API**:

```bash
cargo build --features ffi-api
```

- **Python API (PyO3)**:

```bash
cargo build --features pyo3-api
```

---

## ✅ Why This Structure Works

- **Development separation**: `core-api`, `ffi-api`, `pyo3-api` are independent crates for testing/benchmarking.  
- **Unified publishing**: `streaming-crypto` is the only crate published to crates.io, with optional features.  
- **Flexibility**: We can target Rust, C, or Python from one published crate, while keeping dev ergonomics clean.  

---

## 📂 Revised Project Structure

We don’t need to redefine the FFI and PyO3 wrappers inside `streaming-crypto`. We can **import them directly from `ffi-api` and `pyo3-api` and re‑export under feature flags**. This way, `ffi-api` and `pyo3-api` remain the authoritative sources, while `streaming-crypto` acts as the unified release crate.

---

## 📝 streaming-crypto/src/lib.rs

```rust
// --- MODULES DYNAMIC START ---
// (empty in dev mode, because core-api/ffi-api/pyo3-api are separate crates)
// --- MODULES DYNAMIC END ---

/// Encrypts data by XORing each byte with 0xAA.
///
/// # Examples
///
/// ```
/// use streaming_crypto::encrypt;
///
/// let data = vec![1, 2, 3];
/// let encrypted = encrypt(&data);
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
/// assert_eq!(encrypted[1], 2 ^ 0xAA);
/// assert_eq!(encrypted[2], 3 ^ 0xAA);
/// ```
#[cfg(feature = "core-api")]
pub use core_api::encrypt; // re-export everything from core_api

/// FFI wrapper for encryption.
///
/// # Safety
/// Returns a raw pointer. Caller must manage memory.
///
/// FFI wrapper for encryption.
///
/// # Safety
/// This function returns a raw pointer. The caller must manage memory.
///
/// # Examples
///
/// ```
/// use std::slice;
/// use ffi_api::encrypt;
///
/// let data = vec![1, 2, 3];
/// let ptr = encrypt(data.as_ptr(), data.len());
/// let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
/// ```
#[cfg(feature = "ffi-api")]
pub use ffi_api::encrypt; // re-export the FFI wrapper

/// # Examples
///
/// ```
/// use pyo3::prelude::*;
/// use pyo3_api::encrypt;
/// use pyo3::types::PyBytes;
///
/// Python::with_gil(|py| {
///     let data = PyBytes::new_bound(py, &[1, 2, 3]);
///     
///     let encrypted = encrypt(py, &data).unwrap();
/// 
///     assert_eq!(encrypted[0], 1 ^ 0xAA);
///     assert_eq!(encrypted[1], 2 ^ 0xAA);
///     assert_eq!(encrypted[2], 3 ^ 0xAA);
/// });
/// ```
#[cfg(feature = "pyo3-api")]
pub use pyo3_api::encrypt; // re-export the PyO3 wrapper

#[cfg(feature = "pyo3-api")]
pub use pyo3_api::streaming_crypto; // re-export the #[pymodule]
```

---

## ✅ Advantages

- **No duplication**: FFI and PyO3 wrappers live only in their respective crates.  
- **Single source of truth**: `ffi-api` and `pyo3-api` own their bindings.  
- **Publishable crate stays clean**: `streaming-crypto` just re‑exports under features.  
- **Flexible dev workflow**: We can test/benchmark `core-api`, `ffi-api`, and `pyo3-api` independently, while `streaming-crypto` remains the unified release target.  

---

## 🚀 Usage (1)

- Rust users:

```bash
cargo add streaming-crypto
```

- FFI users:

```bash
cargo build -p streaming-crypto --features ffi-api
```

- Python users:

```bash
cargo build -p streaming-crypto --features pyo3-api
```

---

## **GitHub Actions CI workflow** that makes our repo production‑grade

- Build and test all three subprojects (`core-api`, `ffi-api`, `pyo3-api`).  
- Ensure the publishable crate (`streaming-crypto`) compiles with each feature set (`core-api`, `ffi-api`, `pyo3-api`).  
- Run on Linux, macOS, and Windows for cross‑platform validation.  
- Cache dependencies to speed up builds.  

---

## 📂 File: `.github/workflows/ci.yml`

```yaml
# **CI workflow** for our project structure (`workspace = streaming-crypto`, crate at `streaming-crypto/streaming-crypto`).
# **Build/test commands**: `cargo build -p crate-name` works from the workspace root.

name: CI

### 🔑 Key points
# - **two steps** (`Ensure dev features block` and `Ensure dev dependencies block`) that guarantee the dev blocks are present in `Cargo.toml` during CI runs.  
# - This way, CI always tests against the dev configuration with path dependencies.  
# - The publish workflow will later replace those blocks with the publish versions.
```

---

## ✅ What This Workflow Ensures

- **Cross‑platform builds**: Linux, macOS, Windows.  
- **Feature coverage**: Builds `streaming-crypto` with `core-api`, `ffi-api`, and `pyo3-api`.  
- **Dev crates validated**: `core-api`, `ffi-api`, `pyo3-api` all build/test independently.  
- **Caching**: Faster CI runs by reusing cargo registry.  

---

## 📂 File: `.github/workflows/prepare-publish.yml`

```yaml
name: Prepare Publishable Crate

```

---

## 📂 File: `.github/workflows/publish-crates.yml`

```yaml
### The strategy
# - **Cargo version stays fixed** until we actually change `core-api` (or other Rust sources).
# - **GitHub tags carry extra suffixes** to distinguish retries for PyPI or crates.io, without altering Cargo.toml.
# - Each workflow trims the suffix to get the “real” Cargo version for validation, but uses the full tag for uniqueness in GitHub.

### Tag naming convention
# - Crates.io:  
#   - `v0.1.0-alpha.0-crates.0` → first attempt  
#   - `v0.1.0-alpha.0-crates.1` → retry (same Cargo version, different GitHub tag)
# - PyPI:  
#   - `v0.1.0-alpha.0-pypi.0` → first attempt  
#   - `v0.1.0-alpha.0-pypi.1` → retry

# Here:
# - `0.1.0-alpha.0` = Cargo version (matches Cargo.toml)  
# - `-crates` or `-pypi` = workflow selector  
# - `.N` = retry counter (any integer, ensures unique GitHub tag)

### Workflow changes
# Both workflows need to:
# 1. **Trim suffixes** (`-crates.N` or `-pypi.N`) to extract the Cargo version.
# 2. **Validate Cargo.toml version** against that trimmed version.
# 3. For PyPI, **verify crates.io has published that version** before proceeding.

# ✅ This way:
# - We can retry PyPI publishes as many times as needed (`-pypi.0`, `-pypi.1`, …) without touching Cargo.toml.
# - Crates.io publishes only when we actually bump the Cargo version.
# - Both workflows stay consistent with Cargo.toml, and GitHub tags remain unique.

name: Publish Crates.io
```

---

## 📂 File: `.github/workflows/publish-pypi.yml`

```yml
### The strategy
# - **Cargo version stays fixed** until we actually change `core-api` (or other Rust sources).
# - **GitHub tags carry extra suffixes** to distinguish retries for PyPI or crates.io, without altering Cargo.toml.
# - Each workflow trims the suffix to get the “real” Cargo version for validation, but uses the full tag for uniqueness in GitHub.

### Tag naming convention
# - Crates.io:  
#   - `v0.1.0-alpha.0-crates.0` → first attempt  
#   - `v0.1.0-alpha.0-crates.1` → retry (same Cargo version, different GitHub tag)
# - PyPI:  
#   - `v0.1.0-alpha.0-pypi.0` → first attempt  
#   - `v0.1.0-alpha.0-pypi.1` → retry

# Here:
# - `0.1.0-alpha.0` = Cargo version (matches Cargo.toml)  
# - `-crates` or `-pypi` = workflow selector  
# - `.N` = retry counter (any integer, ensures unique GitHub tag)

### Workflow changes
# Both workflows need to:
# 1. **Trim suffixes** (`-crates.N` or `-pypi.N`) to extract the Cargo version.
# 2. **Validate Cargo.toml version** against that trimmed version.
# 3. For PyPI, **verify crates.io has published that version** before proceeding.

# ✅ This way:
# - We can retry PyPI publishes as many times as needed (`-pypi.0`, `-pypi.1`, …) without touching Cargo.toml.
# - Crates.io publishes only when we actually bump the Cargo version.
# - Both workflows stay consistent with Cargo.toml, and GitHub tags remain unique.

name: Publish PyPI
```

---

## 📂 Project Layout (extended)

```bash
streaming-crypto/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── ffi.rs
│   └── py.rs
└── tests/
    ├── core_api.rs
    ├── ffi_api.rs
    └── pyo3_api.rs
```

---

## 📝 streaming-crypto/tests/core_api.rs

```rust
#[cfg(feature = "core-api")]
#[test]
fn test_encrypt_core_wrapper() {
    let data = vec![1, 2, 3];
    let encrypted = streaming_crypto::encrypt(&data);
    assert_eq!(encrypted.len(), 3);
    // sanity check: XOR with 0xAA
    assert_eq!(encrypted[0], 1 ^ 0xAA);
}
```

---

## 📝 streaming-crypto/tests/ffi_api.rs

```rust
#[cfg(feature = "ffi-api")]
#[test]
fn test_encrypt_ffi_wrapper() {
    use std::slice;

    let data = vec![1, 2, 3];
    let ptr = streaming_crypto::encrypt(data.as_ptr(), data.len());

    // reconstruct slice from raw pointer
    let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };

    assert_eq!(encrypted[0], 1 ^ 0xAA);
    assert_eq!(encrypted[1], 2 ^ 0xAA);
    assert_eq!(encrypted[2], 3 ^ 0xAA);
}
```

---

## 📝 streaming-crypto/tests/pyo3_api.rs

```rust
#[cfg(feature = "pyo3-api")]
#[test]
fn test_encrypt_py_wrapper() {
    use pyo3::Python;
    use pyo3::types::PyBytes;

    Python::with_gil(|py| {
        let data = PyBytes::new_bound(py, &[1, 2, 3]);

        // Pass raw slice
        let encrypted = streaming_crypto::encrypt(py, &data).unwrap();

        assert_eq!(encrypted[0], 1 ^ 0xAA);
        assert_eq!(encrypted[1], 2 ^ 0xAA);
        assert_eq!(encrypted[2], 3 ^ 0xAA);
    });
}
```

---

## ✅ Benefits

- **End‑to‑end validation**: Ensures re‑exports from `core-api`, `ffi-api` and `pyo3-api` are wired correctly.  
- **Feature‑specific testing**: Each feature flag is tested independently.  
- **CI enforcement**: Guarantees that publishing only happens if all re‑exports work.  

---

This way, our **workspace crates remain modular**, but our **publishable crate is fully tested as a façade** before release.  

---

Let’s add **doctests** inside `streaming-crypto/src/lib.rs`. These will serve as **runnable examples** that appear in our published documentation on `crates.io` and `docs.rs`. They also double as tests, since `cargo test` executes doctests automatically.

---

## 📝 streaming-crypto/src/lib.rs (Rust API doctest)

```rust
/// Encrypts data by XORing each byte with 0xAA.
///
/// # Examples
///
/// ```
/// use streaming_crypto::encrypt;
///
/// let data = vec![1, 2, 3];
/// let encrypted = encrypt(&data);
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
/// assert_eq!(encrypted[1], 2 ^ 0xAA);
/// assert_eq!(encrypted[2], 3 ^ 0xAA);
/// ```
#[cfg(feature = "core-api")]
pub use core_api::encrypt; // re-export everything from core_api

/// FFI wrapper for encryption.
///
/// # Safety
/// Returns a raw pointer. Caller must manage memory.
///
/// FFI wrapper for encryption.
///
/// # Safety
/// This function returns a raw pointer. The caller must manage memory.
///
/// # Examples
///
/// ```
/// use std::slice;
/// use ffi_api::encrypt;
///
/// let data = vec![1, 2, 3];
/// let ptr = encrypt(data.as_ptr(), data.len());
/// let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
/// ```
#[cfg(feature = "ffi-api")]
pub use ffi_api::encrypt; // re-export the FFI wrapper

/// # Examples
///
/// ```
/// use pyo3::prelude::*;
/// use pyo3_api::encrypt;
/// use pyo3::types::PyBytes;
///
/// Python::with_gil(|py| {
///     let data = PyBytes::new_bound(py, &[1, 2, 3]);
///     
///     let encrypted = encrypt(py, &data).unwrap();
/// 
///     assert_eq!(encrypted[0], 1 ^ 0xAA);
///     assert_eq!(encrypted[1], 2 ^ 0xAA);
///     assert_eq!(encrypted[2], 3 ^ 0xAA);
/// });
/// ```
#[cfg(feature = "pyo3-api")]
pub use pyo3_api::encrypt; // re-export the PyO3 wrapper
```

---

## ✅ Benefits (1)

- **Documentation + testing unified**: Examples double as tests.  
- **Discoverability**: Users see runnable examples on crates.io/docs.rs.  
- **CI enforcement**: Ensures docs stay correct and compilable.  
- **Cross‑language clarity**: Rust, FFI, and Python usage all demonstrated.  

---

This makes our published crate **self‑documenting and self‑validating**. Every feature flag has examples that are tested automatically.  

---

### 1. Repository setup

We need to **create the GitHub repository in advance** on GitHub.com.  

- Push our initial codebase there.  
- In the repo settings, add our **secrets** (`CARGO_REGISTRY_TOKEN`, `PYPI_API_TOKEN`) under **Settings → Secrets and variables → Actions**.  
- This ensures our workflow has the environment it needs when we eventually tag a release.

---

### 2. Controlling publish triggers

```bash
# See .github/Workflows.md
```

---
