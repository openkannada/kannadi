#!/bin/sh
# Prints the wasm-bindgen version locked in Cargo.lock; fails unless there is exactly 1.
set -eu
lock="$(cd "$(dirname "$0")/.." && pwd)/Cargo.lock"
versions=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print }' "$lock")
if [ "$(printf '%s\n' "$versions" | grep -c .)" -ne 1 ]; then
  echo "Expected 1 wasm-bindgen version in Cargo.lock, found: ${versions:-none}" >&2
  exit 1
fi
printf '%s\n' "$versions"
