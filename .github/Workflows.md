# GitHub Workflows + Actions

## Running GitHub Actions Locally with `gh act`

This guide explains how to install **GitHub CLI (`gh`)**, add the **`gh-act` extension**, and run GitHub Actions workflows (like `ci.yml`) locally on macOS, Windows, or Linux.

---

## 📦 1. Install GitHub CLI (`gh`)

### macOS

1. Download the latest binary release from [GitHub CLI releases](https://cli.github.com/).
2. Extract the archive:

   ```bash
   tar -xvf gh_<version>_macOS_amd64.tar.gz
   ```

3. Move the binary to the PATH:

   ```bash
   sudo mv gh_<version>_macOS_amd64/bin/gh /usr/local/bin/
   ```

4. Verify installation:

   ```bash
   gh --version
   ```

### Linux

1. Download the binary release for Linux from [GitHub CLI releases](https://cli.github.com/).
2. Extract and move the binary:

   ```bash
   tar -xvf gh_<version>_linux_amd64.tar.gz
   sudo mv gh_<version>_linux_amd64/bin/gh /usr/local/bin/
   ```

3. Verify:

   ```bash
   gh --version
   ```

### Windows

1. Download the `.zip` release for Windows from [GitHub CLI releases](https://cli.github.com/).
2. Extract the archive.
3. Move `gh.exe` into a folder included in the PATH (e.g., `C:\Program Files\GitHub CLI\`).
4. Verify in PowerShell:

   ```powershell
   gh --version
   ```

---

## 🔑 2. Authenticate GitHub CLI

Run:

  ```bash
  gh auth login
  ```

* Choose **GitHub.com**.
* Select the preferred protocol (**HTTPS** or **SSH**).
* Authenticate via **web browser** (recommended) or paste a Personal Access Token.

---

## 📥 3. Install `gh-act` Extension

Install the extension that lets run GitHub Actions locally:

```bash
gh extension install https://github.com/nektos/gh-act
```

Verify installation:

```bash
gh act --help
```

---

## 🐳 4. Install and Run Docker

`gh act` requires Docker to simulate GitHub-hosted runners.

* **macOS**: Install Docker Desktop [(docker.com in Bing)](https://www.bing.com/search?q="https%3A%2F%2Fwww.docker.com%2Fproducts%2Fdocker-desktop%2F").
* **Windows**: Install Docker Desktop (ensure WSL2 backend is enabled).
* **Linux**: Install Docker via the package manager:

  ```bash
  sudo apt-get update
  sudo apt-get install docker.io
  ```

* Start Docker and verify:

  ```bash
  docker ps
  ```

---

## ▶️ 5. Run Workflows Locally

### Run all jobs in `ci.yml`

```bash
gh act
```

### Run a specific job

```bash
gh act -j build-test
```

### Simulate a push event

```bash
gh act push
```

### Simulate a tag push (custom event)

Create `push-tag.json`:

```json
{
  "ref": "refs/tags/v1.2.3",
  "repository": {
    "owner": { "login": "github-username" },
    "name": "github-repo"
  },
  "pusher": { "name": "github-username" }
}
```

Run:

```bash
gh act -j detect-tag --eventpath push-tag.json
```

---

## 🔐 6. Handling Secrets

If the workflow uses secrets, create a `.secrets` file:

```bash
MY_SECRET=super_secret_value
```

Run with:

```bash
gh act --secret-file .secrets
```

---

## ⚙️ 7. Choosing Runner Images

On first run, `gh act` will ask which Docker image to use:

* **Large (~17GB)**: Full GitHub runner snapshot.
* **Medium (~500MB)**: Common tools (recommended).
* **Micro (<200MB)**: Minimal, NodeJS only.

Choose **Medium** unless the workflow requires many preinstalled tools.

---

## 🔧 Steps to Run `detect-tag` with a Push Tag

1. **Make sure Docker is running**  
   `gh act` won’t work without Docker Desktop running and the socket available.

2. **Simulate a push event with a tag**  
   Use the `--eventpath` option to provide a custom event JSON file that mimics a GitHub push with a tag.

   Example: create a file `.github/push-tag.json` in our repo root:

   ```json
   {
     "ref": "refs/tags/v1.2.3",
     "repository": {
       "owner": { "login": "DreamzIt02" },
       "name": "streaming-crypto"
     },
     "pusher": { "name": "DreamzIt02" }
   }
   ```

3. **Run the job with that event payload**  

   ```bash
    gh act -j detect-tag --eventpath .github/push-tag.json
   ```

   This tells `gh act` to simulate a push event where the ref is a tag (`refs/tags/v1.2.3`).

4. **Alternative: pass event type directly**

   If our workflow is triggered by `on: push: tags:`, we can also run:

   ```bash
   gh act push -j detect-tag
   ```

   But without an `--eventpath`, it defaults to a generic push event (usually `refs/heads/main`). That’s why the custom JSON is more reliable for tag-specific jobs.

---

## 🔧 Steps to Run `prepare-publish` with a Docker Volume

Let's make `actions/upload-artifact` and `actions/download-artifact` behave differently when running under **act**, and store artifacts in a shared host-mounted directory instead of GitHub’s artifact service.

## 🧪 2️⃣ Run act With Shared Volume

Always run act like this:

```bash
mkdir -p .act-artifacts

gh act -j prepare-publish \
  -e .github/push-tag.json \
  --artifact-server-path .act-artifacts \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest

# Or

gh act push \
  -e .github/push-tag.json \
  --artifact-server-path .act-artifacts \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest
```

Now every job container shares `/.act-artifacts`.

---

## 🧹 3️⃣ Clean Up Between Runs

```bash
rm -rf .act-artifacts/*
```

---

## Script (bash)

```bash
mkdir -p .act-artifacts
mkdir -p .act-cache

gh act -j publish-pypi \
  -e .github/push-tag.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest

rm -rf .act-artifacts/*
rm -rf .act-cache/*
```

---

## ✅ Step 1 — Create Optimized Image

Create:

```bash
docker/ubuntu-linux-custom/Dockerfile
```

```Dockerfile
ARG ARCH=ghcr.io/catthehacker/ubuntu:act-latest
FROM ${ARCH}

ENV CARGO_HOME=/cargo
ENV RUSTUP_HOME=/rustup
ENV PATH="$CARGO_HOME/bin:$PATH"

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    python3.11 \
    python3.11-venv \
    python3.11-dev \
    python3.12 \
    python3.12-venv \
    python3.12-dev \
    python3.13 \
    python3.13-venv \
    python3.13-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

# Python virtual environment
RUN python3.11 -m venv /opt/venv \
    && /opt/venv/bin/pip install --upgrade pip \
    && /opt/venv/bin/pip install maturin cibuildwheel twine build

RUN python3.12 -m venv /opt/venv \
    && /opt/venv/bin/pip install --upgrade pip \
    && /opt/venv/bin/pip install maturin cibuildwheel twine build

RUN python3.13 -m venv /opt/venv \
    && /opt/venv/bin/pip install --upgrade pip \
    && /opt/venv/bin/pip install maturin cibuildwheel twine build

ENV PATH="/opt/venv/bin:$PATH"
```

```Dockerfile
ARG ARCH=quay.io/pypa/manylinux_2_28_x86_64
FROM ${ARCH}

ENV CARGO_HOME=/cargo
ENV RUSTUP_HOME=/rustup
ENV PATH="$CARGO_HOME/bin:$PATH"
ENV PYBIN=/opt/python/cp312-cp312/bin

RUN yum install -y \
        curl \
        git \
        make \
        gcc \
        gcc-c++ \
        pkg-config \
        openssl-devel \
    && yum clean all \
    && rm -rf /var/cache/yum

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN $PYBIN/pip install --upgrade pip \
    && $PYBIN/pip install maturin cibuildwheel twine build
```

```Dockerfile
ARG ARCH=quay.io/pypa/musllinux_1_2_x86_64
FROM ${ARCH}

ENV CARGO_HOME=/cargo
ENV RUSTUP_HOME=/rustup
ENV PATH="$CARGO_HOME/bin:$PATH"
ENV PYBIN=/opt/python/cp312-cp312/bin

RUN apk add --no-cache \
        curl \
        git \
        make \
        gcc \
        g++ \
        pkgconfig \
        openssl-dev

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && rustup target add x86_64-unknown-linux-musl

RUN $PYBIN/pip install --upgrade pip \
    && $PYBIN/pip install maturin cibuildwheel twine build
```

---

## ✅ Step 2 — Build Image Once

```bash
docker build -t ubuntu-linux-custom \
  --build-arg ARCH=ghcr.io/catthehacker/ubuntu:act-latest \
  -f docker/ubuntu-linux-custom/Dockerfile .

docker build -t manylinux-custom-x86_64 \
  --build-arg ARCH=quay.io/pypa/manylinux_2_28_x86_64 \
  -f docker/many-linux-custom/Dockerfile .

docker build -t manylinux-custom-aarch64 \
  --platform linux/arm64 \
  --build-arg ARCH=quay.io/pypa/manylinux_2_28_aarch64 \
  -f docker/many-linux-custom/Dockerfile .

docker build -t musllinux-custom-x86_64 \
  --build-arg ARCH=quay.io/pypa/musllinux_1_2_x86_64 \
  -f docker/musl-linux-custom/Dockerfile .

docker build -t musllinux-custom-aarch64 \
  --platform linux/arm64 \
  --build-arg ARCH=quay.io/pypa/musllinux_1_2_aarch64 \
  -f docker/musl-linux-custom/Dockerfile .
```

This may take 5–6 minutes.

But only once.

---

## ✅ Step 3 — Use It in Act

```bash
gh act push \
  --workflows .github/workflows/ci-core.yml \
  -e .github/push-tag-core.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

gh act push \
  --workflows .github/workflows/ci-ffi.yml \
  -e .github/push-tag-ffi.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

gh act push \
  --workflows .github/workflows/ci-pyo3.yml \
  -e .github/push-tag-pyo3.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

rm -rf .act-artifacts/*
rm -rf .act-cache/*
```

### Push main with tag [vN.N.N-*-crates.N] (run publish-crates.yml)

```bash
mkdir -p .act-artifacts
mkdir -p .act-cache

# Run only detect-tag from publish-crates.yml
gh act -j detect-tag \
  --workflows .github/workflows/publish-crates.yml \
  -e .github/push-tag-crates.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

# Run only prepare-publish from publish-crates.yml
gh act -j prepare \
  --workflows .github/workflows/publish-crates.yml \
  -e .github/push-tag-crates.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --reuse \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

# Run only publish-crates from publish-crates.yml
gh act push \
  --workflows .github/workflows/publish-crates.yml \
  -e .github/push-tag-crates.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

rm -rf .act-artifacts/*
rm -rf .act-cache/*
```

### Push main with tag [vN.N.N-*-pypi.N] (run publish-pypi.yml)

```bash
mkdir -p .act-artifacts
mkdir -p .act-cache

# Run only detect-tag from publish-pypi.yml
gh act -j detect-tag \
  --workflows .github/workflows/publish-pypi.yml \
  -e .github/push-tag-pypi.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

# Run only prepare-publish from publish-pypi.yml
gh act -j prepare \
  --workflows .github/workflows/publish-pypi.yml \
  -e .github/push-tag-pypi.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --reuse \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest

# Run full publish-pypi.yml
gh act push \
  --workflows .github/workflows/publish-pypi.yml \
  -e .github/push-tag-pypi.json \
  --artifact-server-path .act-artifacts \
  --cache-server-path .act-cache \
  --container-daemon-socket /var/run/docker.sock \
  --pull=false \
  -P ubuntu-latest=ubuntu-linux-custom:latest \
  -P macos-latest=ubuntu-linux-custom:latest \
  -P windows-latest=ubuntu-linux-custom:latest

rm -rf .act-artifacts/*
rm -rf .act-cache/*
```

---

## 🚀 What This Fixes

| Before                   | After   |
| ------------------------ | ------- |
| 4m Rust install          | 0s      |
| rustup network downloads | none    |
| cargo install maturin    | none    |
| CI time ~6 min           | ~40–60s |

---

## 🧠 Optional: Make It Ultra-Fast

If we want elite-level speed:

### Mount cargo cache from host

```bash
gh act push \
  -P ubuntu-latest=ubuntu-linux-custom \
  --bind \
  --reuse
```

Or manually mount:

```bash
-v ~/.cargo:/root/.cargo
-v ~/.rustup:/root/.rustup
```

Then toolchains never reinstall.

---

## 🎯 Extra Optimization (Optional)

We are cloning actions every job:

```bash
git clone actions/cache
git clone actions/setup-python
```

To avoid that:

```bash
gh act push --action-offline-mode
```

This uses cached action repos.

---

## 🏁 Final Result

After optimization our pipeline will:

* Start instantly
* Skip rust download
* Skip maturin install
* Reuse toolchain
* Finish in under 1 minute

---
