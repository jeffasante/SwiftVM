#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

out1="$(cd "$ROOT" && cargo run -q -p swiftvm-cli -- --once apps/demo/native_bridge.svm)"
[[ "$out1" == "SWIFT BRIDGE" ]] || {
  echo "unexpected SVM bridge output: $out1" >&2
  exit 1
}

out2="$(cd "$ROOT" && cargo run -q -p swiftvm-cli -- --once apps/demo/native_bridge.swift)"
[[ "$out2" == "SWIFT FROM SWIFT" ]] || {
  echo "unexpected Swift bridge output: $out2" >&2
  exit 1
}

echo "native bridge once regression: OK"
