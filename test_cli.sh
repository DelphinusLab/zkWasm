#!/bin/bash

set -e

rm -rf params output

cargo build --release

CLI=./target/release/delphinus-cli

$CLI --params ./params wasm_output setup -k 18 --wasm crates/zkwasm/wasm/wasm_output.wasm
$CLI --params ./params wasm_output dry-run --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output
$CLI --params ./params wasm_output prove --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output --mock
$CLI --params ./params wasm_output verify --output ./output
