#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

out="$(cd "$ROOT" && cargo run -q -p swiftvm-cli -- --once apps/demo/object_props.swift)"
[[ "$out" == "42" ]] || {
  echo "unexpected object props output: $out" >&2
  exit 1
}

echo "object props regression: OK"
