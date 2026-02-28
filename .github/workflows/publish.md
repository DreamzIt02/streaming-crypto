# ✅ 1️⃣ Strict SemVer (No Leading Zeros)

Official SemVer rule:

* `0` is valid
* `1, 2, 3…` valid
* `01, 002, 0001` ❌ invalid

```yaml
- name: Validate strict SemVer tag
  shell: bash
  run: |
    TAG="${GITHUB_REF_NAME}"

    echo "Detected tag: $TAG"

    # Strict SemVer 2.0.0 (no leading zeros)
    SEMVER_REGEX='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

    if [[ ! "$TAG" =~ $SEMVER_REGEX ]]; then
      echo "❌ Invalid strict SemVer tag."
      echo "Expected: vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-PRERELEASE"
      echo "No leading zeros allowed."
      exit 1
    fi

    echo "✅ Valid strict SemVer tag."
```

---

## 🎯 Now This Rejects

| Tag       | Result  |
| --------- | ------- |
| `v01.2.3` | ❌      |
| `v1.02.3` | ❌      |
| `v1.2.03` | ❌      |
| `v1.2`    | ❌      |
| `v1`      | ❌      |

---

## ✅ Still Allows

* `v0.1.0`
* `v1.2.3`
* `v1.2.3-alpha`
* `v1.2.3-alpha.0`
* `v10.20.30-rc.3`

---

## ✅ 2️⃣ Ensure Workflow NEVER Runs Without Tag

A job-level guard:

```yaml
name: Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest
```

This ensures:

* Even if someone edits the trigger later
* Even if GitHub behavior changes
* Even if manually triggered incorrectly

The job will NOT run unless it is a tag push.

---

## 🏆 Final Hardened Version (Production-Grade)

```yaml
name: Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Validate strict SemVer tag
        shell: bash
        run: |
          TAG="${GITHUB_REF_NAME}"

          SEMVER_REGEX='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

          if [[ ! "$TAG" =~ $SEMVER_REGEX ]]; then
            echo "Invalid strict SemVer tag"
            exit 1
          fi

      - name: Ensure tag matches Cargo.toml version
        working-directory: streaming-crypto
        run: |
          TAG_VERSION="${GITHUB_REF_NAME#v}"
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)

          if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version does not match Cargo.toml"
            exit 1
          fi
```

---

## 🔐 What We Now Have

* ✔ Strict SemVer 2.0.0
* ✔ No leading zeros
* ✔ Pre-release support
* ✔ No accidental branch publishing
* ✔ Tag must match Cargo.toml
* ✔ Fully production safe

---

## ✅ Option A — Publish Pre-Releases to crates.io (Fully Valid)

`crates.io` supports SemVer pre-releases:

* `1.0.0-alpha`
* `1.0.0-beta.1`
* `1.0.0-rc.3`

They behave correctly:

* `cargo add streaming-crypto` → installs latest stable
* `cargo add streaming-crypto --prerelease` → installs pre-release
* `cargo update` will not automatically upgrade stable → pre-release

So this is completely safe and normal.

---

## ✅ Option B — Publish Only Stable to crates.io, Pre-Release to GitHub Releases

Some projects prefer:

* `v1.2.3-alpha` → GitHub Release only
* `v1.2.3` → crates.io + GitHub Release

Reasons teams choose this:

* Avoid polluting crates.io with unstable APIs
* Keep crates.io clean for production users
* Use GitHub Releases for testing builds

---

## 🎯 If We Want Option B

Add this condition:

```yaml
- name: Detect pre-release
  id: prerelease_check
  run: |
    if [[ "${GITHUB_REF_NAME}" == *-* ]]; then
      echo "is_prerelease=true" >> $GITHUB_OUTPUT
    else
      echo "is_prerelease=false" >> $GITHUB_OUTPUT
    fi
```

Then guard publishing:

```yaml
- name: Publish crate
  if: steps.prerelease_check.outputs.is_prerelease == 'false'
  working-directory: streaming-crypto
  run: cargo publish --allow-dirty
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

Now:

| Tag          | crates.io | GitHub Release |
| ------------ | --------- | -------------- |
| v1.2.3       | ✅        | ✅             |
| v1.2.3-alpha | ❌        | ✅             |
| v1.2.3-rc.1  | ❌        | ✅             |

---

## 🧠 Which Should *We* Choose?

According to our architecture (workspace, FFI, PyO3, CI vendoring):

If:

* We’re actively iterating API → **Publish pre-releases to crates.io**
* We want stable ecosystem signal → **Only publish stable**

Most Rust ecosystem libraries (tokio, serde, etc.) publish pre-releases to crates.io.

So publishing pre-releases to crates.io is completely normal and professional.

---

👉 Publish pre-releases to crates.io.

It gives us:

* Ecosystem integration
* Early feedback
* Cleaner upgrade path
* No special CI branching logic

---

We want:

* ✅ `vX.Y.Z-alpha` → auto publish to crates.io (pre-release)
* ✅ `vX.Y.Z` → auto publish + GitHub Release
* ✅ Auto changelog generation
* ✅ Automatic version bump enforcement
* ✅ Strict SemVer

Below is a complete, clean architecture.

---

## 🏗 Final Release Architecture

## 🔹 Tag-Based Release Model

| Tag              | crates.io | GitHub Release | Notes            |
| ---------------- | --------- | -------------- | ---------------- |
| `v1.2.3-alpha.1` | ✅        | ❌             | Pre-release only |
| `v1.2.3`         | ✅        | ✅             | Stable release   |

---

## 🧠 How It Works

1. Developer bumps version in:

   ```bash
   streaming-crypto/Cargo.toml
   ```

2. Commit:

   ```bash
   chore: release 1.2.3-alpha.1
   ```

3. Push tag:

   ```bash
   git tag v1.2.3-alpha.1
   git push origin v1.2.3-alpha.1
   ```

4. Workflow:

   * Validates strict SemVer
   * Ensures tag matches Cargo.toml
   * Vendors core-api
   * Publishes to crates.io
   * If stable → creates GitHub Release + changelog

---

## 🚀 Full Production `publish.yml`

```yaml
name: Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # needed for changelog generation

      # -------------------------------------------------
      # 1️⃣ Strict SemVer validation (no leading zeros)
      # -------------------------------------------------
      - name: Validate strict SemVer tag
        run: |
          TAG="${GITHUB_REF_NAME}"

          SEMVER_REGEX='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

          if [[ ! "$TAG" =~ $SEMVER_REGEX ]]; then
            echo "Invalid strict SemVer tag"
            exit 1
          fi

      # -------------------------------------------------
      # 2️⃣ Enforce Cargo.toml version match
      # -------------------------------------------------
      - name: Ensure tag matches Cargo.toml version
        working-directory: streaming-crypto
        run: |
          TAG_VERSION="${GITHUB_REF_NAME#v}"
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)

          if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version does not match Cargo.toml"
            exit 1
          fi

      # -------------------------------------------------
      # 3️⃣ Detect pre-release
      # -------------------------------------------------
      - name: Detect pre-release
        id: prerelease_check
        run: |
          if [[ "${GITHUB_REF_NAME}" == *-* ]]; then
            echo "is_prerelease=true" >> $GITHUB_OUTPUT
          else
            echo "is_prerelease=false" >> $GITHUB_OUTPUT
          fi

      # -------------------------------------------------
      # 4️⃣ Vendor core-api
      # -------------------------------------------------
      - name: Vendor core-api
        run: |
          rm -rf streaming-crypto/src/core_api
          mkdir -p streaming-crypto/src/core_api
          cp -r core-api/src/* streaming-crypto/src/core_api/
          mv streaming-crypto/src/core_api/lib.rs \
             streaming-crypto/src/core_api/mod.rs

          # Remove path dependency
          sed -i.bak '/core-api/d' streaming-crypto/Cargo.toml

      # -------------------------------------------------
      # 5️⃣ Package validation
      # -------------------------------------------------
      - name: Package validation
        working-directory: streaming-crypto
        run: cargo package --allow-dirty

      # -------------------------------------------------
      # 6️⃣ Publish to crates.io
      # -------------------------------------------------
      - name: Publish crate
        working-directory: streaming-crypto
        run: cargo publish --allow-dirty
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      # -------------------------------------------------
      # 7️⃣ Generate changelog (stable only)
      # -------------------------------------------------
      - name: Generate changelog
        if: steps.prerelease_check.outputs.is_prerelease == 'false'
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
        if: steps.prerelease_check.outputs.is_prerelease == 'false'
        uses: softprops/action-gh-release@v2
        with:
          body_path: CHANGELOG_TEMP.md
        env:
          GITHUB_TOKEN: ${{ secrets.CARGO_GITHUB_TOKEN }}
```

---

## 🔥 What This Gives Us

## Pre-release tag (`v1.2.3-alpha.1`)

* ✔ Strict validation
* ✔ Version enforced
* ✔ Vendored
* ✔ Published to crates.io
* ❌ No GitHub Release

---

## Stable tag (`v1.2.3`)

* ✔ Strict validation
* ✔ Version enforced
* ✔ Vendored
* ✔ Published to crates.io
* ✔ Auto changelog generated
* ✔ GitHub Release created automatically

---

## 🛡 Automatic Version Bump Enforcement

Already enforced here:

```bash
if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
```

This prevents:

* Forgetting to bump Cargo.toml
* Tagging wrong version
* Publishing mismatched version

---

## 🏆 Final Result

We now have:

* Production-grade release pipeline
* Strict SemVer enforcement
* Clean pre-release strategy
* Automatic changelog
* Automatic GitHub Release
* No accidental publishing
* Workspace-safe vendoring

---
