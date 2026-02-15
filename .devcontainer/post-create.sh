#!/usr/bin/env bash
set -euo pipefail

# Pre-fetch cargo dependencies so first build is faster
cargo fetch

# Install clippy and rustfmt (may already be present in base image)
rustup component add clippy rustfmt 2>/dev/null || true

# Pull the default test image used by E2E tests and .dual.toml
docker pull node:20 || echo "warn: docker pull failed — E2E tests will pull on first run"
