#!/bin/bash

set -e
set -x

rm -rf output/*.data

# Single test
RUST_LOG=info cargo run --release --features cuda -- -k 21 --function zkmain --output ./output --wasm ttttt_bg.wasm setup

RUST_LOG=info cargo run --release --features cuda -- -k 21 --function zkmain --output ./output --wasm ttttt_bg.wasm single-prove
RUST_LOG=info cargo run --release --features cuda -- -k 21 --function zkmain --output ./output --wasm ttttt_bg.wasm single-verify --proof output/zkwasm.0.transcript.data --instance output/zkwasm.0.instance.data
exit 0
RUST_LOG=info cargo run --release --features cuda -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm aggregate-prove --public 133:i64 --public 2:i64
RUST_LOG=info cargo run --release --features cuda -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm aggregate-verify --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
if [ -d "sol" ]; then
  RUST_LOG=info cargo run --release --features cuda -- -k 18 --function zkmain --output ./output --wasm wasm/wasm_output.wasm solidity-aggregate-verifier --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
fi
