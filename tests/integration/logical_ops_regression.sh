#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
out="$(cd "$ROOT" && cargo run -q -p swiftvm-cli -- --once apps/demo/logical_ops.swift)"

[[ "$out" == "1" ]] || {
  echo "unexpected logical ops output: $out" >&2
  exit 1
}

echo "logical ops regression: OK"
