#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FRONTEND_BIN="$ROOT/swift/Frontend/.build/debug/swiftvm-frontend"
TMP_SWIFT="$(mktemp /tmp/swiftvm-frontend-regression-XXXX.swift)"
trap 'rm -f "$TMP_SWIFT"' EXIT

cat > "$TMP_SWIFT" <<'SWIFT'
var sum: Int = 0

func main() -> Int {
    sum = 0
    for i in 0..<6 {
        sum = sum + i
    }
    return sum
}
SWIFT

if [[ ! -x "$FRONTEND_BIN" ]]; then
  echo "frontend binary missing at $FRONTEND_BIN" >&2
  echo "build it with: xcrun swift build --package-path $ROOT/swift/Frontend" >&2
  exit 1
fi

OUTPUT="$($FRONTEND_BIN "$TMP_SWIFT")"

echo "$OUTPUT" | rg -q '^func main\(\)$'
echo "$OUTPUT" | rg -q 'load_global sum'
echo "$OUTPUT" | rg -q 'load_var i'
echo "$OUTPUT" | rg -q '^  add$'
echo "$OUTPUT" | rg -q '^  store_global sum$'

echo "frontend for-loop assignment regression: OK"
