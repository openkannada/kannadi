#!/bin/sh
# Builds the typing engine for the browser into the directory given as $1
# (default target/typing-web): the WebAssembly package and the model file.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-"$root/target/typing-web"}

want=$("$root/web/wasm-bindgen-version.sh")
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$have" != "$want" ]; then
  echo "wasm-bindgen $want is needed (found '${have:-none}')." >&2
  echo "Install it: cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

cargo build -p kannadi-typing-wasm --lib --profile wasm --locked --target wasm32-unknown-unknown \
  --manifest-path "$root/Cargo.toml"
rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" \
  "${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/wasm/kannadi_typing_wasm.wasm"
cp "$root/crates/kannadi-typing/model/kn.typing" "$out/"
wasm=$(wc -c < "$out/kannadi_typing_wasm_bg.wasm" | tr -d ' ')
model=$(wc -c < "$out/kn.typing" | tr -d ' ')
echo "Built $out: $wasm bytes of wasm ($(gzip -9 -c "$out/kannadi_typing_wasm_bg.wasm" | wc -c | tr -d ' ') gzipped), $model bytes of model ($(gzip -9 -c "$out/kn.typing" | wc -c | tr -d ' ') gzipped)"
