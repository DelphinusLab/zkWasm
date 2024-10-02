#!/bin/bash

CLI=./target/release/zkwasm-cli

set -e
set -x

test_continuation_cli() {
    cargo build --release --features continuation,perf,profile,cuda
    rm -rf params/*.data params/*.config output
    $CLI --params ./params context setup --host standard
    $CLI --params ./params context dry-run --wasm crates/zkwasm/wasm/rust-sdk-test.wasm --public 25:i64 --private 1:i64 --ctxin 1:i64 --output ./output
    CUDA_VISIBLE_DEVICES=0 $CLI --params ./params context prove --public 25:i64 --private 1:i64 --ctxin 1:i64 --padding 3 --wasm crates/zkwasm/wasm/rust-sdk-test.wasm --output ./output
    $CLI --params ./params context verify --output ./output
}


#x=50
#while [ $x -gt 0 ]; do
#    test_phantom_cli
    test_continuation_cli
#    x=$(($x-1))
#done

