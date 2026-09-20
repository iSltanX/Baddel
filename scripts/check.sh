#!/usr/bin/env bash
# Phase gate: Rust tests + lints, frontend type-check, frontend build.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/env.sh

cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run build
