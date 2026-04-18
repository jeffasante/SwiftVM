#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  run-sim.sh --project <path/to/App.xcodeproj> --scheme <Scheme> --bundle-id <com.example.app> [--configuration Debug|Release] [--derived-data <path>]

Builds the app for iOS Simulator, installs on a booted simulator (or boots one), and launches it.
USAGE
}

PROJECT=""
SCHEME=""
BUNDLE_ID=""
CONFIGURATION="Debug"
DERIVED_DATA=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project) PROJECT="$2"; shift 2 ;;
    --scheme) SCHEME="$2"; shift 2 ;;
    --bundle-id) BUNDLE_ID="$2"; shift 2 ;;
    --configuration) CONFIGURATION="$2"; shift 2 ;;
    --derived-data) DERIVED_DATA="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "$PROJECT" || -z "$SCHEME" || -z "$BUNDLE_ID" ]]; then
  usage
  exit 1
fi

if [[ -z "$DERIVED_DATA" ]]; then
  SAFE_SCHEME="$(echo "$SCHEME" | tr ' /' '__')"
  DERIVED_DATA="/tmp/swiftvm-derived-${SAFE_SCHEME}"
fi

echo "[1/4] Building $SCHEME ($CONFIGURATION) for iOS Simulator..."
xcodebuild \
  -project "$PROJECT" \
  -scheme "$SCHEME" \
  -configuration "$CONFIGURATION" \
  -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$DERIVED_DATA" \
  CODE_SIGNING_ALLOWED=NO \
  build >/tmp/swiftvm-run-sim-build.log

APP_PATH="$(find "$DERIVED_DATA/Build/Products" -path "*/${CONFIGURATION}-iphonesimulator/*.app" | head -n 1)"
if [[ -z "$APP_PATH" ]]; then
  echo "Could not find built .app under $DERIVED_DATA/Build/Products" >&2
  exit 1
fi

BOOTED_UDID="$(xcrun simctl list devices available | awk -F '[()]' '/Booted/{print $2; exit}')"
if [[ -z "$BOOTED_UDID" ]]; then
  BOOTED_UDID="$(xcrun simctl list devices available | awk -F '[()]' '/iPhone/{print $2; exit}')"
  if [[ -z "$BOOTED_UDID" ]]; then
    echo "No available simulator devices found." >&2
    exit 1
  fi
  echo "[2/4] Booting simulator device $BOOTED_UDID..."
  xcrun simctl boot "$BOOTED_UDID" || true
else
  echo "[2/4] Using booted simulator $BOOTED_UDID..."
fi

echo "[3/4] Installing $APP_PATH..."
xcrun simctl install "$BOOTED_UDID" "$APP_PATH"

echo "[4/4] Launching $BUNDLE_ID..."
LAUNCH_OUT="$(xcrun simctl launch "$BOOTED_UDID" "$BUNDLE_ID")"
echo "$LAUNCH_OUT"

echo "Done."
echo "Build log: /tmp/swiftvm-run-sim-build.log"
