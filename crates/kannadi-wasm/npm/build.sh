#!/bin/sh
# Builds the npm package into target/npm/kannadi and packs it: crates/kannadi-wasm/npm/build.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
target=${CARGO_TARGET_DIR:-$root/target}
mkdir -p "$target"
target=$(cd "$target" && pwd)
out="$target/npm/kannadi"

want=$("$root/web/wasm-bindgen-version.sh")
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$have" != "$want" ]; then
  echo "wasm-bindgen $want is needed (found '${have:-none}')." >&2
  echo "Install it: cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
if [ "$version" != "$(node -p 'require(process.argv[1]).version' "$here/package.json")" ]; then
  echo "package.json must have the workspace version $version." >&2
  exit 1
fi

cargo build -p kannadi-wasm --lib --profile wasm --locked --target wasm32-unknown-unknown \
  --manifest-path "$root/Cargo.toml"

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" "$target/wasm32-unknown-unknown/wasm/kannadi_wasm.wasm"
cp "$here/package.json" "$here/README.md" "$here/node.mjs" "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$out/"
(cd "$out" && npm pack --pack-destination "$target/npm")
