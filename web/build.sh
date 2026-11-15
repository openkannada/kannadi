#!/bin/sh
# Builds the converter page into web/dist (or the directory given as $1).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-"$root/web/dist"}

if [ -e "$out" ] && [ ! -d "$out/pkg" ] && [ -n "$(ls -A "$out")" ]; then
  echo "$out is not empty and is not a previous build; choose another directory." >&2
  exit 1
fi

want=$("$root/web/wasm-bindgen-version.sh")
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
cp -R "$root"/web/src/. "$out/"
node "$root/web/site/build.mjs" "$out" "${KANNADI_SITE_URL:-}"
echo "Built $out ($(wc -c < "$out/pkg/kannadi_wasm_bg.wasm" | tr -d ' ') bytes of wasm)"
