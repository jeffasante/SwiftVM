#!/usr/bin/env bash
set -euo pipefail

# SwiftVM Hot-Reload Dev Server
# Usage: ./dev.sh [source_file_or_directory]
# Default source: apps/demo/main.svm
#
# When given a directory, the CLI:
#   - Scans ALL .swift files for logic (functions, state)
#   - Extracts SwiftUI view config (titles, labels, text) → /tmp/swiftvm-viewconfig.json
#   - Watches the entire directory for changes

cd "$(dirname "$0")"

ARG="${1:-apps/demo/main.svm}"

# Validate the path exists
if [[ ! -e "$ARG" ]]; then
    echo "error: path not found: $ARG" >&2
    exit 1
fi

# For directories, verify there are Swift files inside
if [[ -d "$ARG" ]]; then
    swift_count=$(find "$ARG" -type f -name "*.swift" ! -name "Package.swift" | head -1 | wc -l)
    if [[ "$swift_count" -eq 0 ]]; then
        echo "error: no .swift files found in: $ARG" >&2
        exit 1
    fi
    MODE="project"
    LABEL="$ARG (project)"
else
    MODE="file"
    LABEL="$ARG"
fi

echo "╔═════════════════════════════════════════════════════════╗"
echo "║   SwiftVM Hot-Reload Dev Environment (by jeffasante)    ║"
echo "╚═════════════════════════════════════════════════════════╝"
echo ""

# Step 1: Build the VM engine + Swift frontend
echo "[1/3] Building SwiftVM engine..."
cargo build -q -p swiftvm-cli
echo "      ✓ Engine built"

# Step 1b: Auto-inject SwiftVMHook.swift into the project (if project mode)
if [[ "$MODE" == "project" ]]; then
    HOOK_SRC="tools/SwiftVMHook.swift"
    # Find the App/ or Sources/ directory (common Xcode project layouts)
    APP_DIR=$(find "$ARG" -maxdepth 2 -type d -name "App" | head -1)
    if [[ -z "$APP_DIR" ]]; then
        APP_DIR=$(find "$ARG" -maxdepth 2 -type d -name "Sources" | head -1)
    fi
    if [[ -z "$APP_DIR" ]]; then
        APP_DIR="$ARG"
    fi

    if [[ -f "$HOOK_SRC" ]]; then
        HOOK_DST="$APP_DIR/SwiftVMHook.swift"
        if [[ ! -f "$HOOK_DST" ]] || ! diff -q "$HOOK_SRC" "$HOOK_DST" > /dev/null 2>&1; then
            cp "$HOOK_SRC" "$HOOK_DST"
            echo "      ✓ Injected SwiftVMHook.swift → $HOOK_DST"
        fi

        # Also copy the ObjC boot file (ensures auto-start)
        BOOT_SRC="tools/SwiftVMHookBoot.m"
        BOOT_DST="$APP_DIR/SwiftVMHookBoot.m"
        if [[ -f "$BOOT_SRC" ]]; then
            if [[ ! -f "$BOOT_DST" ]] || ! diff -q "$BOOT_SRC" "$BOOT_DST" > /dev/null 2>&1; then
                cp "$BOOT_SRC" "$BOOT_DST"
                echo "      ✓ Injected SwiftVMHookBoot.m → $BOOT_DST"
            fi
        fi

        # Auto-add to Xcode target if xcodeproj found
        XCODEPROJ=$(find "$ARG" -maxdepth 1 -name "*.xcodeproj" | head -1)
        if [[ -n "$XCODEPROJ" && -f "tools/inject-hook.rb" ]]; then
            # Detect the main app target (first native target)
            TARGET_NAME=$(ruby -r xcodeproj -e "
                p = Xcodeproj::Project.open('$XCODEPROJ')
                t = p.targets.find { |t| t.product_type.to_s.include?('application') }
                puts t&.name || p.targets.first&.name
            " 2>/dev/null)
            if [[ -n "$TARGET_NAME" ]]; then
                ruby tools/inject-hook.rb "$XCODEPROJ" "$TARGET_NAME" "$HOOK_DST" 2>/dev/null
                if [[ -f "$BOOT_DST" ]]; then
                    ruby tools/inject-hook.rb "$XCODEPROJ" "$TARGET_NAME" "$BOOT_DST" 2>/dev/null
                fi
            fi
        fi
    fi
fi

# Step 2: Build and install the iOS app (optional)
if [[ -f "test-reload/run-sim.sh" ]]; then
    echo "[2/3] Building & installing iOS app..."
    bash test-reload/run-sim.sh
    echo "      ✓ App running in Simulator"
else
    echo "[2/3] Skipping iOS Simulator (test-reload/run-sim.sh not found)"
fi

# Step 3: Start the dev server
echo "[3/3] Starting hot-reload dev server..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Source: $LABEL"
if [[ "$MODE" == "project" ]]; then
echo "  Mode:   Project (all .swift files)"
echo "  Config: /tmp/swiftvm-viewconfig.json"
fi
echo "  Save any file → updates instantly"
echo "  Keys: r = reload  R = hard reload  q = quit"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
exec cargo run -q -p swiftvm-cli -- "$ARG"
