#!/bin/bash

CLI=./target/release/delphinus-cli

set -e
set -x

CUDA="--features cuda"

test_default_cli() {
    cargo build --release $CUDA
    rm -rf params output
    $CLI --params ./params wasm_output setup --wasm ./crates/zkwasm/wasm/wasm_output.wasm
    $CLI --params ./params wasm_output dry-run --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output
    $CLI --params ./params wasm_output prove --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output
    $CLI --params ./params wasm_output verify --output ./output
}

test_uniform_circuit_cli() {
    cargo build --release --features uniform-circuit $CUDA
    rm -rf params output
    $CLI --params ./params wasm_output setup
    $CLI --params ./params wasm_output dry-run --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output
    $CLI --params ./params wasm_output prove --wasm crates/zkwasm/wasm/wasm_output.wasm --public 133:i64 --public 2:i64 --output ./output
    $CLI --params ./params wasm_output verify --output ./output
}

test_continuation_cli() {
RUSTFLAGS=-g cargo build --release --features continuation $CUDA
rm -rf out.perf-folded
rm -rf perf.svg
#    rm -rf params output
#    $CLI --params ./params wasm_output setup
time $CLI --params ./params wasm_output dry-run --wasm main.wasm --output ./output  --private 10:i64 --private 32:i64 --public 42:i64
time perf record  -g  --call-graph=dwarf  $CLI --params ./params wasm_output prove --wasm main.wasm --output ./output -m --private 10:i64 --private 32:i64 --public 42:i64
perf script | ./FlameGraph/stackcollapse-perf.pl > out.perf-folded
./FlameGraph/flamegraph.pl out.perf-folded > perf.svg
#    $CLI --params ./params wasm_output verify --output ./output
}

#test_default_cli
#test_uniform_circuit_cli
test_continuation_cli

