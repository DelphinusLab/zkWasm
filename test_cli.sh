#!/bin/bash

set -e
set -x

rm -rf output

# Single test
RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm setup &&
RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm single-prove --public 3:i64 &&
RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm single-verify --public 3:i64 --proof output/zkwasm.0.transcript.data

RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm aggregate-prove --public 3:i64
RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm aggregate-verify --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
RUST_LOG=info ./target/release/cli --function bsearch --output ./output --wasm wasm/bsearch_64.wasm solidity-aggregate-verifier --proof output/aggregate-circuit.0.transcript.data  --instances output/aggregate-circuit.0.instance.data
