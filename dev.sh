#!/usr/bin/env bash
set -euo pipefail

# SwiftVM Hot-Reload Dev Server
# Usage: ./dev.sh [source_file]
# Default source: test-reload/test-reload/Logic.swift

cd "$(dirname "$0")"

SOURCE="${1:-test-reload/test-reload/Logic.swift}"

echo "╔══════════════════════════════════════════╗"
echo "║   SwiftVM Hot-Reload Dev Environment     ║"
echo "╚══════════════════════════════════════════╝"
echo ""

# Step 1: Build the VM engine
echo "[1/3] Building SwiftVM engine..."
cargo build -q -p swiftvm-cli
echo "      ✓ Engine built"

# Step 2: Build and install the iOS app
echo "[2/3] Building & installing iOS app..."
bash test-reload/run-sim.sh
echo "      ✓ App running in Simulator"

# Step 3: Start the dev server watching your file
echo "[3/3] Starting hot-reload dev server..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Edit: $SOURCE"
echo "  Save the file → App updates instantly"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
exec cargo run -q -p swiftvm-cli -- "$SOURCE"
