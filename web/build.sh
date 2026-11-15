#!/bin/sh
# Builds the converter page into web/dist (or the directory given as $1).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-"$root/web/dist"}

want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print }' "$root/Cargo.lock")
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$have" != "$want" ]; then
  echo "wasm-bindgen $want is needed (found '${have:-none}')." >&2
  echo "Install it: cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

cargo build -p kannadi-wasm --lib --profile wasm --locked --target wasm32-unknown-unknown \
  --manifest-path "$root/Cargo.toml"

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out/pkg" \
  "${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/wasm/kannadi_wasm.wasm"
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Os --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o "$out/pkg/kannadi_wasm_bg.wasm" "$out/pkg/kannadi_wasm_bg.wasm"
fi
cp "$root"/web/src/* "$out/"
echo "Built $out ($(wc -c < "$out/pkg/kannadi_wasm_bg.wasm" | tr -d ' ') bytes of wasm)"
