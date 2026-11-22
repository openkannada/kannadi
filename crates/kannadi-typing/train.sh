#!/usr/bin/env bash
# Rebuilds model/kn.typing and prints the evaluation tables.
# DATA must hold the files fetched by the README's "Training data" steps.
set -euo pipefail

DATA=${DATA:?set DATA to the folder with the training data}
HERE=$(cd "$(dirname "$0")" && pwd)
LEX=$DATA/dakshina/dakshina_dataset_v1.0/kn/lexicons
ROM=$DATA/dakshina/dakshina_dataset_v1.0/kn/romanized
OUT=${OUT:-$HERE/model/kn.typing}

cargo build --release -p kannadi-typing --no-default-features --features train
BIN=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/release/kannadi-typing-train

# Dakshina's dev and test words are kept out of the training pairs.
"$BIN" build \
  --aksharantar "$DATA/aksharantar/kan_train.json" \
  --dakshina "$LEX/kn.translit.sampled.train.tsv" \
  --exclude "$LEX/kn.translit.sampled.dev.tsv" --exclude "$LEX/kn.translit.sampled.test.tsv" \
  --counts "$DATA/wiki-train.tsv" --min-word-count 5 --pair-word-count 5 \
  --sentences "$DATA/wiki-sentences.tsv" --min-pair 5 --max-pairs 30000 \
  --order 4 --min-count 1,2,4,8 \
  --out "$OUT" "$@"

# Weights are tuned on the dev words and the first 10,000 words of the dev sentences.
"$BIN" tune --model "$OUT" --out "$OUT" \
  --dev "$LEX/kn.translit.sampled.dev.tsv" \
  --context-native "$ROM/kn.romanized.rejoined.dev.native.txt" \
  --context-roman "$ROM/kn.romanized.rejoined.dev.roman.txt" --context-limit 10000

echo "Dakshina test words:"
"$BIN" eval --model "$OUT" --test "$LEX/kn.translit.sampled.test.tsv"
echo "Dakshina test sentences, each word after the one before it:"
"$BIN" eval --model "$OUT" \
  --context-native "$ROM/kn.romanized.rejoined.test.native.txt" \
  --context-roman "$ROM/kn.romanized.rejoined.test.roman.txt"
