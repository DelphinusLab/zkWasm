#!/bin/bash

set -e
set -x

rm -rf output/*.data

# Single test
RUST_LOG=info cargo run --release --features cuda -- --host default -k 18 --function zkmain --param ./params --output ./output --wasm ../zkwasm/wasm/wasm_output.wasm setup
RUST_LOG=info cargo run --release --features cuda -- --host default -k 18 --function zkmain --param ./params --output ./output --wasm ../zkwasm/wasm/wasm_output.wasm checksum

RUST_LOG=info cargo run --release --features cuda -- --host default -k 18 --function zkmain --param ./params --output ./output --wasm ../zkwasm/wasm/wasm_output.wasm single-prove --public 133:i64 --public 2:i64
RUST_LOG=info cargo run --release --features cuda -- --host default -k 18 --function zkmain --param ./params --output ./output --wasm ../zkwasm/wasm/wasm_output.wasm single-verify
