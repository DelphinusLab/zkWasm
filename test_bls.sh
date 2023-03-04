#!/bin/bash

set -e
set -x

# rm -rf output

# Single test
RUST_LOG=info cargo run --release  -- -k 20 --function zkmain --output ./output --wasm ctest/bls.wasm setup

RUST_LOG=info cargo run --release  -- -k 20 --function zkmain --output ./output --wasm ctest/bls.wasm single-prove
RUST_LOG=info cargo run --release  -- -k 20 --function zkmain --output ./output --wasm ctest/bls.wasm single-verify --proof output/zkwasm.0.transcript.data
RUST_LOG=info cargo run --release  -- -k 20 --function zkmain --output ./output --wasm ctest/bls.wasm aggregate-prove
RUST_LOG=info cargo run --release  -- -k 20 --function zkmain --output ./output --wasm ctest/bls.wasm aggregate-verify --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
