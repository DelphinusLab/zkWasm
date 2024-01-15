#!/bin/bash

set -x
set -e

rm -rf params output

cargo run --release -- --params ./params wasm_output setup -k 18 --wasm crates/zkwasm/wasm/wasm_output.wasm
cargo run --release -- --params ./params wasm_output dry-run --public 133:i64 --public 2:i64 --output ./output
cargo run --release -- --params ./params wasm_output prove --public 133:i64 --public 2:i64 --output ./output --mock
cargo run --release -- --params ./params wasm_output verify --output ./output
