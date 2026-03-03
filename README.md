# Streaming Crypto

[![CI](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci.yml/badge.svg)](https://github.com/DreamzIt02/streaming-crypto/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/streaming-crypto.svg)](https://crates.io/crates/streaming-crypto)
[![Docs.rs](https://docs.rs/streaming-crypto/badge.svg)](https://docs.rs/streaming-crypto)
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

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
  workflow_dispatch: # <-- allows manual runs

jobs:
  build-test:
    name: Build & Test (Rust + FFI + PyO3)
    runs-on: ${{ matrix.os }}

    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable]

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ matrix.rust }}

      - name: Install Python
        id: setup-python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'
          
      - name: Export PYO3_PYTHON
        shell: bash
        run: echo "PYO3_PYTHON=${{ steps.setup-python.outputs.python-path }}" >> $GITHUB_ENV

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      # Ensure dev manifest blocks are in place
      - name: Ensure dev features block exists
        working-directory: streaming-crypto
        shell: bash
        run: |
          grep -q "# --- FEATURES DYNAMIC START ---" Cargo.toml || { echo "Missing FEATURES DYNAMIC START block"; exit 1; }
          grep -q "# --- FEATURES DYNAMIC END ---" Cargo.toml || { echo "Missing FEATURES DYNAMIC END block"; exit 1; }

      - name: Ensure dev dependencies block exists
        working-directory: streaming-crypto
        shell: bash
        run: |
          grep -q "# --- DEPENDENCIES DYNAMIC START ---" Cargo.toml || { echo "Missing DEPENDENCIES DYNAMIC START block"; exit 1; }
          grep -q "# --- DEPENDENCIES DYNAMIC END ---" Cargo.toml || { echo "Missing DEPENDENCIES DYNAMIC END block"; exit 1; }

      - name: Build core-api
        run: cargo build -p core-api

      - name: Test core-api
        run: cargo test -p core-api

      - name: Build ffi-api
        run: cargo build -p ffi-api

      - name: Test ffi-api
        run: cargo test -p ffi-api

      - name: Build pyo3-api
        run: cargo build -p pyo3-api

      - name: Test pyo3-api
        run: cargo test -p pyo3-api

      - name: Workspace check (core-api)
        run: cargo check --workspace --no-default-features --features core-api

      - name: Workspace check (ffi-api)
        run: cargo check --workspace --no-default-features --features ffi-api
      
      - name: Workspace check (pyo3-api)
        run: cargo check --workspace --no-default-features --features pyo3-api

      - name: Test workspace (core-api)
        run: cargo test --workspace --no-default-features --features core-api

      - name: Test workspace (ffi-api)
        run: cargo test --workspace --no-default-features --features ffi-api

      - name: Test workspace (pyo3-api)
        run: cargo test --workspace --no-default-features --features pyo3-api

      - name: Build streaming-crypto (Rust API default)
        run: cargo build -p streaming-crypto --no-default-features --features core-api

      - name: Build streaming-crypto (FFI API feature)
        run: cargo build -p streaming-crypto --no-default-features --features ffi-api

      - name: Build streaming-crypto (PyO3 API feature)
        run: cargo build -p streaming-crypto --no-default-features --features pyo3-api

      - name: Run integration tests streaming-crypto re-exports (Rust API)
        run: cargo test -p streaming-crypto --no-default-features --features core-api

      - name: Run integration tests streaming-crypto re-exports (FFI API)
        run: cargo test -p streaming-crypto --no-default-features --features ffi-api

      - name: Run integration tests streaming-crypto re-exports (PyO3 API)
        run: cargo test -p streaming-crypto --no-default-features --features pyo3-api
      
      - name: Run doctests (Rust API)
        run: cargo test -p streaming-crypto --no-default-features --features core-api --doc

      - name: Run doctests (FFI API)
        run: cargo test -p streaming-crypto --no-default-features --features ffi-api --doc

      - name: Run doctests (PyO3 API)
        run: cargo test -p streaming-crypto --no-default-features --features pyo3-api --doc

### 🔑 Key points
# - **two steps** (`Ensure dev features block` and `Ensure dev dependencies block`) that guarantee the dev blocks are present in `Cargo.toml` during CI runs.  
# - This way, CI always tests against the dev configuration with path dependencies.  
# - The publish workflow will later replace those blocks with the publish versions.
```

- We can minimize copy loops if there no difference amongst the sub-projects:

```yml
for crate in core-api ffi-api pyo3-api; do
  rm -rf streaming-crypto/streaming-crypto/src/$crate
  mkdir -p streaming-crypto/streaming-crypto/src/$crate/src
  cp -r $crate/src/* streaming-crypto/streaming-crypto/src/$crate/src/
  cp $crate/README.md streaming-crypto/streaming-crypto/src/$crate/README.md
  cp $crate/LICENSE streaming-crypto/streaming-crypto/src/$crate/LICENSE
done
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

on:
  workflow_call:
    inputs:
      tag:
        required: true
        type: string
      modules:
        required: false
        type: string  # semi-colon separated string,
      features:
        required: false
        type: string  # semi-colon separated string,
      dependencies:
        required: false
        type: string  # semi-colon separated string,

jobs:
  prepare-publish:
    name: Prepare publishable crate
    runs-on: ubuntu-latest

    steps:
      - name: require tag_version input
        run: |
          if [ -z "${{ inputs.tag }}" ]; then
            echo "No tag_version provided — refusing to run."
            exit 0
          fi

      - name: Mark local run
        if: ${{ env.ACT == 'true' }}
        run: echo "IS_LOCAL=true" >> $GITHUB_ENV

      # -------------------------------------------------
      # 1️⃣ Checkout repository
      # -------------------------------------------------
      - name: Checkout repository
        uses: actions/checkout@v4

      # -------------------------------------------------
      # 2️⃣ Vendors to publish crate
      # -------------------------------------------------
      - name: Debug directory structure
        run: |
          echo "PWD:"; pwd
          echo "Root contents:"; ls -la
          echo "streaming-crypto:"; ls -la streaming-crypto

      # -------------------------------------------------
      # ✅ Copy core-api into publishable crate
      # -------------------------------------------------
      - name: Copy core-api sources + metadata
        run: |
          rm -rf streaming-crypto/src/core_api
          mkdir -p streaming-crypto/src/core_api

          # Copy src folder contents
          cp -r core-api/src/* streaming-crypto/src/core_api/

          # Copy crate-level metadata (optional)
          cp core-api/README.md streaming-crypto/src/core_api/README.md || true
          cp core-api/.gitignore streaming-crypto/src/core_api/.gitignore || true

          # Rename lib.rs → mod.rs
          mv streaming-crypto/src/core_api/lib.rs \
            streaming-crypto/src/core_api/mod.rs

      # -------------------------------------------------
      # ✅ Copy ffi-api into publishable crate
      # -------------------------------------------------
      - name: Copy ffi-api sources + metadata
        run: |
          rm -rf streaming-crypto/src/ffi_api
          mkdir -p streaming-crypto/src/ffi_api

          # Copy src folder contents
          cp -r ffi-api/src/* streaming-crypto/src/ffi_api/

          # Copy crate-level metadata (optional)
          cp ffi-api/README.md streaming-crypto/src/ffi_api/README.md || true
          cp ffi-api/.gitignore streaming-crypto/src/ffi_api/.gitignore || true

          # Rename lib.rs → mod.rs
          mv streaming-crypto/src/ffi_api/lib.rs \
            streaming-crypto/src/ffi_api/mod.rs

      # -------------------------------------------------
      # ✅ Copy pyo3-api into publishable crate
      # -------------------------------------------------
      - name: Copy pyo3-api sources + metadata
        run: |
          rm -rf streaming-crypto/src/pyo3_api
          mkdir -p streaming-crypto/src/pyo3_api

          # Copy src folder contents
          cp -r pyo3-api/src/* streaming-crypto/src/pyo3_api/

          # Copy crate-level metadata (optional)
          cp pyo3-api/README.md streaming-crypto/src/pyo3_api/README.md || true
          cp pyo3-api/.gitignore streaming-crypto/src/pyo3_api/.gitignore || true

          # Rename lib.rs → mod.rs
          mv streaming-crypto/src/pyo3_api/lib.rs \
            streaming-crypto/src/pyo3_api/mod.rs
      
      # -------------------------------------------------
      # 3️⃣ ✅ Safe Replace: MODULES block
      # -------------------------------------------------
      - name: Replace modules block for publish
        working-directory: streaming-crypto/src
        shell: bash
        run: |
          FILE="lib.rs"
          START="// --- MODULES DYNAMIC START ---"
          END="// --- MODULES DYNAMIC END ---"

          [ -f "$FILE" ] || { echo "$FILE not found"; exit 1; }
          [ "$(grep -cF "$START" "$FILE")" -eq 1 ] || { echo "Invalid START marker"; exit 1; }
          [ "$(grep -cF "$END" "$FILE")" -eq 1 ] || { echo "Invalid END marker"; exit 1; }

          awk -v start="$START" \
              -v end="$END" \
              -v dynamic="${{ inputs.modules }}" '
            BEGIN { in_block=0 }

            $0 ~ start {
              in_block=1
              print start

              # Base publish modules (optional file)
              if (system("test -f ../Cargo.publish.modules") == 0) {
                system("cat ../Cargo.publish.modules")
                print ""     # <-- FORCE newline boundary
              }

              # Dynamic modules input (optional)
              if (dynamic != "") {
                printf "%s\n", dynamic
              }

              next
            }

            $0 ~ end {
              in_block=0
              print end
              next
            }

            in_block { next }
            { print }
          ' "$FILE" > "$FILE.tmp"

          mv "$FILE.tmp" "$FILE"
      # -------------------------------------------------
      # 3️⃣ ✅ Safe Replace: FEATURES block
      # -------------------------------------------------
      - name: Replace features block for publish
        working-directory: streaming-crypto
        shell: bash
        run: |
          FILE="Cargo.toml"
          START="# --- FEATURES DYNAMIC START ---"
          END="# --- FEATURES DYNAMIC END ---"

          [ -f "$FILE" ] || { echo "$FILE not found"; exit 1; }
          [ "$(grep -cF "$START" "$FILE")" -eq 1 ] || { echo "Invalid START marker"; exit 1; }
          [ "$(grep -cF "$END" "$FILE")" -eq 1 ] || { echo "Invalid END marker"; exit 1; }

          awk -v start="$START" \
              -v end="$END" \
              -v dynamic="${{ inputs.features }}" '
            BEGIN { in_block=0 }

            $0 ~ start {
              in_block=1
              print start

              if (system("test -f Cargo.publish.features") == 0) {
                system("cat Cargo.publish.features")
                print ""     # <-- FORCE newline boundary
              }

              if (dynamic != "") {
                printf "%s\n", dynamic
              }

              next
            }

            $0 ~ end {
              in_block=0
              print end
              next
            }

            in_block { next }
            { print }
          ' "$FILE" > "$FILE.tmp"

          mv "$FILE.tmp" "$FILE"

      # -------------------------------------------------
      # 4️⃣ ✅ Safe Replace: DEPENDENCIES block
      # -------------------------------------------------
      - name: Replace dependencies block for publish
        working-directory: streaming-crypto
        shell: bash
        run: |
          FILE="Cargo.toml"
          START="# --- DEPENDENCIES DYNAMIC START ---"
          END="# --- DEPENDENCIES DYNAMIC END ---"

          [ -f "$FILE" ] || { echo "Cargo.toml not found"; exit 1; }
          [ "$(grep -cF "$START" "$FILE")" -eq 1 ] || { echo "Invalid START marker"; exit 1; }
          [ "$(grep -cF "$END" "$FILE")" -eq 1 ] || { echo "Invalid END marker"; exit 1; }

          awk -v start="$START" \
              -v end="$END" \
              -v dynamic="${{ inputs.dependencies }}" '
            BEGIN { in_block=0 }

            $0 ~ start {
              in_block=1
              print start

              if (system("test -f Cargo.publish.dependencies") == 0) {
                system("cat Cargo.publish.dependencies")
                print ""     # <-- FORCE newline boundary
              }

              if (dynamic != "") {
                printf "%s\n", dynamic
              }

              next
            }

            $0 ~ end {
              in_block=0

              # 3️⃣ Common workspace deps
              if (system("test -f ../Cargo.toml") == 0) {
                print ""     # <-- FORCE newline boundary
                system("sed -n \"/# --- DEPENDENCIES COMMON START ---/,/# --- DEPENDENCIES COMMON END ---/p\" ../Cargo.toml")
              }

              print end
              next
            }

            in_block { next }
            { print }
          ' "$FILE" > "$FILE.tmp"

          mv "$FILE.tmp" "$FILE"

      # -------------------------------------------------
      # 5️⃣ Prepare & upload crate for publishing
      # -------------------------------------------------
      - name: Prepare crate for publishing
        run: |
          mkdir -p prepared-crate
          cp -r streaming-crypto/* prepared-crate/

      - name: Upload prepared crate
        uses: actions/upload-artifact@v4
        with:
          name: prepared-crate
          path: prepared-crate
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

on:
  push:
    tags:
      - 'v*-crates.[0-9]'

jobs:
  detect-tag:
    runs-on: ubuntu-latest
    outputs:
      tag_version: ${{ steps.version_detect.outputs.tag_version }}
      tag_is_valid: ${{ steps.version_detect.outputs.tag_is_valid }}
      is_prerelease: ${{ steps.prerelease_check.outputs.is_prerelease }}
    steps:
      - uses: actions/checkout@v4

      # -------------------------------------------------
      # 1️⃣ Strict SemVer validation + capture version (with -crates.N suffix)
      # -------------------------------------------------
      - name: Validate strict SemVer tag (with -crates.N suffix) & capture tag version
        id: version_detect
        run: |
          RAW_TAG="${GITHUB_REF_NAME}"
          # Remove leading "v"
          BASE="${RAW_TAG#v}"
          # Trim trailing "-crates.N" (suffix with retry counter)
          BASE_TAG=$(echo "$BASE" | sed -E 's/-crates\.[0-9]+$//')

          SEMVER_REGEX='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

          # Export the trimmed version for later steps
          if [[ "$BASE_TAG" =~ $SEMVER_REGEX ]]; then
            echo "tag_version=$BASE_TAG" >> $GITHUB_OUTPUT
            echo "tag_is_valid=true" >> $GITHUB_OUTPUT
          else
            echo "Invalid strict SemVer tag: $RAW_TAG (parsed as $BASE_TAG)"
            echo "tag_is_valid=false" >> $GITHUB_OUTPUT
          fi

      # -------------------------------------------------
      # 2️⃣ Enforce Cargo.toml version match
      # -------------------------------------------------
      - name: Ensure tag matches Cargo.toml version
        if: ${{ steps.version_detect.outputs.tag_is_valid == 'true' }}
        working-directory: streaming-crypto
        run: |
          TAG_VERSION="${{ steps.version_detect.outputs.tag_version }}"
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)
          
          if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version -> $TAG_VERSION does not match Cargo.toml-> $CARGO_VERSION"
            exit 1
          fi
      
      # -------------------------------------------------
      # 3️⃣ Detect pre-release
      # -------------------------------------------------
      - name: Detect pre-release
        if: ${{ steps.version_detect.outputs.tag_is_valid == 'true' }}
        id: prerelease_check
        run: |
          TAG_VERSION="${{ steps.version_detect.outputs.tag_version }}"
          if [[ "$TAG_VERSION" == *-* ]]; then
            echo "is_prerelease=true" >> $GITHUB_OUTPUT
          else
            echo "is_prerelease=false" >> $GITHUB_OUTPUT
          fi

  # ✅ prepare-publish
  prepare:
    if: ${{ needs.detect-tag.outputs.tag_is_valid == 'true' }}
    needs: [detect-tag]
    uses: ./.github/workflows/prepare-publish.yml
    with:
      tag: ${{ needs.detect-tag.outputs.tag_version }}
      modules: |
        #[cfg(feature = \"core-api\")] pub mod core_api;
        #[cfg(feature = \"ffi-api\")]  pub mod ffi_api;
        #[cfg(feature = \"pyo3-api\")] pub mod pyo3_api;
      features: |
        default = [\"core-api\"]

  # ✅ publish to crates.io
  publish:
    if: startsWith(github.ref, 'refs/tags/')
    name: Publish to crates.io + github release
    runs-on: ubuntu-latest
    needs: [detect-tag, prepare] # Require both

    steps:
      - name: Mark local run
        if: ${{ env.ACT == 'true' }}
        run: echo "IS_LOCAL=true" >> $GITHUB_ENV

      # 1️⃣ Fresh checkout for changelog
      - name: Checkout repository (for git history)
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      # 2️⃣ Get prepared crate for publishing
      - name: Download prepared crate
        uses: actions/download-artifact@v4
        with:
          name: prepared-crate
          path: prepared-crate

      - name: Move prepared crate into repo
        run: |
          rm -rf streaming-crypto
          mv prepared-crate streaming-crypto

      # 3️⃣ Install libraries
      - name: Install Rust
        if: ${{ env.IS_LOCAL != 'true' }}
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install Python
        if: ${{ env.IS_LOCAL != 'true' }}
        id: setup-python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Export PYO3_PYTHON
        if: ${{ env.IS_LOCAL != 'true' }}
        shell: bash
        run: echo "PYO3_PYTHON=${{ steps.setup-python.outputs.python-path }}" >> $GITHUB_ENV

      # 4️⃣ Cache
      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      # -------------------------------------------------
      # 5️⃣ Package validation
      # -------------------------------------------------
      - name: Package validation
        working-directory: streaming-crypto
        shell: bash
        run: cargo package --allow-dirty --list 

      # -------------------------------------------------
      # 6️⃣ Publish to crates.io
      # -------------------------------------------------
      - name: Dry-run publish
        working-directory: streaming-crypto
        shell: bash
        run: cargo publish --allow-dirty --dry-run 

      - name: Publish crate
        if: ${{ env.IS_LOCAL != 'true' }}
        working-directory: streaming-crypto
        shell: bash
        run: cargo publish --allow-dirty 
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      # -------------------------------------------------
      # 7️⃣ Generate changelog (stable only)
      # -------------------------------------------------
      - name: Generate changelog
        if: ${{ needs.detect-tag.outputs.is_prerelease == 'false' }}
        id: changelog
        run: |
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          echo "Previous tag: $PREV_TAG"

          if [ -z "$PREV_TAG" ]; then
            git log --pretty=format:"- %s" > CHANGELOG_TEMP.md
          else
            git log ${PREV_TAG}..HEAD --pretty=format:"- %s" > CHANGELOG_TEMP.md
          fi

      # -------------------------------------------------
      # 8️⃣ Create GitHub Release (stable only)
      # -------------------------------------------------
      - name: Create GitHub Release
        if: ${{ env.IS_LOCAL != 'true' && needs.detect-tag.outputs.is_prerelease == 'false' }}
        uses: softprops/action-gh-release@v2
        with:
          body_path: CHANGELOG_TEMP.md
        env:
          GITHUB_TOKEN: ${{ secrets.CARGO_GITHUB_TOKEN }}
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

on:
  push:
    tags:
      - 'v*-pypi.[0-9]'

jobs:
  detect-tag:
    runs-on: ubuntu-latest
    outputs:
      tag_version: ${{ steps.version_detect.outputs.tag_version }}
      tag_is_valid: ${{ steps.version_detect.outputs.tag_is_valid }}
      is_prerelease: ${{ steps.prerelease_check.outputs.is_prerelease }}
    steps:
      - uses: actions/checkout@v4

      # -------------------------------------------------
      # 1️⃣ Strict SemVer validation + capture version (with -pypi.N suffix)
      # -------------------------------------------------
      - name: Validate strict SemVer tag (with -pypi.N suffix) & capture tag version
        id: version_detect
        run: |
          RAW_TAG="${GITHUB_REF_NAME}"
          # Remove leading "v"
          BASE="${RAW_TAG#v}"
          # Trim trailing "-pypi.N" (suffix with retry counter)
          BASE_TAG=$(echo "$BASE" | sed -E 's/-pypi\.[0-9]+$//')

          SEMVER_REGEX='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

          # Export the trimmed version for later steps
          if [[ "$BASE_TAG" =~ $SEMVER_REGEX ]]; then
            echo "tag_version=$BASE_TAG" >> $GITHUB_OUTPUT
            echo "tag_is_valid=true" >> $GITHUB_OUTPUT
          else
            echo "Invalid strict SemVer tag: $RAW_TAG (parsed as $BASE_TAG)"
            echo "tag_is_valid=false" >> $GITHUB_OUTPUT
          fi

      # -------------------------------------------------
      # 2️⃣ Enforce Cargo.toml version match
      # -------------------------------------------------
      - name: Ensure tag matches Cargo.toml version
        if: ${{ steps.version_detect.outputs.tag_is_valid == 'true' }}
        working-directory: streaming-crypto
        run: |
          TAG_VERSION="${{ steps.version_detect.outputs.tag_version }}"
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)

          if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version -> $TAG_VERSION does not match Cargo.toml-> $CARGO_VERSION"
            exit 1
          fi
      
      # -------------------------------------------------
      # 3️⃣ Detect pre-release
      # -------------------------------------------------
      - name: Detect pre-release
        if: ${{ steps.version_detect.outputs.tag_is_valid == 'true' }}
        id: prerelease_check
        run: |
          TAG_VERSION="${{ steps.version_detect.outputs.tag_version }}"
          if [[ "$TAG_VERSION" == *-* ]]; then
            echo "is_prerelease=true" >> $GITHUB_OUTPUT
          else
            echo "is_prerelease=false" >> $GITHUB_OUTPUT
          fi
      
      # -------------------------------------------------
      # 4️⃣ Verify crates.io version exists
      # -------------------------------------------------
      - name: Verify crates.io version exists
        if: ${{ steps.version_detect.outputs.tag_is_valid == 'true' }}
        run: |
          VERSION="${{ steps.version_detect.outputs.tag_version }}"
          PKG="streaming-crypto"
          URL="https://crates.io/api/v1/crates/${PKG}/${VERSION}"

          # echo "Checking crates.io for $PKG@$VERSION"
          echo "Debug: curl -v -H 'User-Agent: act-ci' $URL"

          # RESPONSE=$(curl -s "https://crates.io/api/v1/crates/$PKG/$VERSION")
          RESPONSE=$(curl -sSf -H "User-Agent: streaming-crypto-ci" "$URL")

          # Print the raw JSON for debugging
          echo "Raw crates.io response:"
          echo "$RESPONSE"

          # If crates.io returns an error, version does not exist
          if echo "$RESPONSE" | jq -e '.errors' > /dev/null; then
            echo "Version $VERSION not yet published to crates.io"
            exit 1
          fi

          NUM=$(echo "$RESPONSE" | jq -r '.version.num')
          echo "Extracted version.num: $NUM"

          if [ "$NUM" = "$VERSION" ]; then
            echo "Version $VERSION exists on crates.io"
          else
            echo "Unexpected response from crates.io"
            exit 1
          fi


  # ✅ prepare-publish
  prepare:
    if: ${{ needs.detect-tag.outputs.tag_is_valid == 'true' }}
    needs: [detect-tag]
    uses: ./.github/workflows/prepare-publish.yml
    with:
      tag: ${{ needs.detect-tag.outputs.tag_version }}
      modules: |
        #[cfg(feature = \"core-api\")] pub mod core_api;
        #[cfg(feature = \"ffi-api\")]  pub mod ffi_api;
        #[cfg(feature = \"pyo3-api\")] pub mod pyo3_api;
      features: |
        default = [\"pyo3-api\"]
      dependencies: |
        core-api = { package=\"streaming-crypto\", version = \"${{ needs.detect-tag.outputs.tag_version }}\", features = [] }

  # ✅ publish to pypi.org
  publish:
    if: startsWith(github.ref, 'refs/tags/')
    name: Build & Publish Python Wheel
    runs-on: ubuntu-latest
    needs: [detect-tag, prepare] # Require both

    steps:
      - name: Mark local run
        if: ${{ env.ACT == 'true' }}
        run: echo "IS_LOCAL=true" >> $GITHUB_ENV
      
      # 1️⃣ Fresh checkout for changelog
      - name: Checkout repository (for git history)
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      # 2️⃣ Get prepared crate for publishing
      - name: Download prepared crate
        uses: actions/download-artifact@v4
        with:
          name: prepared-crate
          path: prepared-crate

      - name: Move prepared crate into repo
        run: |
          rm -rf streaming-crypto
          mv prepared-crate streaming-crypto
      
      # 3️⃣ Install libraries
      - name: Install Rust
        if: ${{ env.IS_LOCAL != 'true' }}
        uses: dtolnay/rust-toolchain@stable

      - name: Install Python
        if: ${{ env.IS_LOCAL != 'true' }}
        id: setup-python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Export PYO3_PYTHON
        if: ${{ env.IS_LOCAL != 'true' }}
        shell: bash
        run: echo "PYO3_PYTHON=${{ steps.setup-python.outputs.python-path }}" >> $GITHUB_ENV

      - name: Install maturin
        if: ${{ env.IS_LOCAL != 'true' }}
        run: pip install maturin

      # ✅ 4️⃣ Build wheel
      - name: Build wheel with PyO3 feature
        working-directory: streaming-crypto
        shell: bash
        run: maturin build --release

      - name: Publish to PyPI
        if: ${{ env.IS_LOCAL != 'true' }}
        working-directory: streaming-crypto
        shell: bash
        run: maturin publish -u __token__ -p ${{ secrets.PYPI_API_TOKEN }}
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

## 📂 streaming-crypto/README.md (with badges)

```markdown
# Rust Project

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

### Rust API (bash)

  `cargo add streaming-crypto`

### FFI API (bash)

  `cargo build --features ffi-api`

### Python API (bash)

  `pip install streaming-crypto`

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
```

---
