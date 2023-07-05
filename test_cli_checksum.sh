#!/bin/bash

set -e
set -x

rm -rf output/*.data

# Single test
RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm setup
RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm checksum

RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm dry-run --public 133:i64 --public 2:i64

RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm single-prove --public 133:i64 --public 2:i64

RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm single-verify --instance output/zkwasm.0.instance.data --proof output/zkwasm.0.transcript.data

RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm aggregate-prove --public 133:i64 --public 2:i64

RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm aggregate-verify --proof output/aggregate-circuit.0.transcript.data --instances output/aggregate-circuit.0.instance.data
if [ -d "sol" ]; then
  RUST_LOG=info cargo run --release --features cuda --features checksum -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm solidity-aggregate-verifier --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
fi