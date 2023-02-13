<p align="center">
  <img src="zkwasm-bk.png" height="100">
</p>

<p align="center">
  <a href="https://github.com/DelphinusLab/zkWasm/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache 2-blue.svg"></a>
</p>

# Overview：

The mission of DelphiusLab is to provide Web2 developpers a consise toolset to leverage the power of Web3 in their applications. The ZKWASM (ZKSNARK virtual machine that supports Web Assembly) is a crucial part which served as a trustless trustless adopter between rich applilcation running on WASM runtime and smart contracts on various blockchains.

WASM (or WebAssembly) is an open standard binary code format close to assembly. Its initial objective is to provide an alternative to java-script with better performance in the current web ecosystems. Benefiting from its platform independence, front-end flexibility (can be compiled from the majority of languages including C, C++, assembly script, rust, etc.), good isolated runtime and speed that is close to native binary, its usage starts to arise in the distributed cloud and edge computing. Recently it has become a popular binary format for users to run customized functions on AWS Lambda, Open Yurt, AZURE, etc.

The idea of ZKWASM, is derived from ZKSNARK (Zero-Knowledge Succinct Non-Interactive Argument of Knowledge) which is a combination of SNARG (Succinct non-interactive arguments) and zero-knowledge proof. In general, the adoption of ZKSNARK usually requires implementing a program in arithmetic circuits or circuit-friendly languages (Pinocchio, TinyRAM, Buffet/Pequin, Geppetto, xJsnark framework, ZoKrates) that forms a barrier for existing programs to leverage the power of it. An alternative way is that instead of applying ZKSNARK, on the source code, we apply it on the bytecode level of a virtual machine and implement a zksnark-backed virtual machine. In this work, we take the approach of writing the whole WASM virtual machine in ZKSNARK circuits so that existing WASM applications can benefit from ZKSNARK by simply running on the ZKWASM, without any modification. Therefore, the cloud service provider can prove to any user that the computation result is computed honestly and no private information is leaked.


# Circuit Details:
https://jhc.sjtu.edu.cn/~hongfeifu/manuscriptb.pdf

# Quick start with ZKWASM command line

## Setup input:
wasm code

## Runtime input:
input of wasm function and the top level function must be main.

## Proving target:
simulation of wasm execution of target wasm bytecode with particular inputs are correct.

# Command line:
## Setup via WASM image:
```
cargo run --release -- --function <FUNCTION_NAME> --wasm <WASM_BINARY> setup [OPTIONS]
```

## Single prove and verify:
```
cargo run --release -- --function <FUNCTION_NAME> --wasm <WASM_BINARY> single-prove [OPTIONS]
cargo run --release -- --function <FUNCTION_NAME> --wasm <WASM_BINARY> single-verify [OPTIONS] --proof <PROOF_PATH>
```
with OPTIONS:
```
    -o, --output [<OUTPUT_PATH>...]
            Path of the output files.
            The md5 of the wasm binary file is the default path if not supplied.

        --private [<PRIVATE_INPUT>...]
            Private arguments of your wasm program arguments of format value:type where
            type=i64|bytes|bytes-packed, multiple values should be separated with ','

        --public [<PUBLIC_INPUT>...]
            Public arguments of your wasm program arguments of format value:type where
            type=i64|bytes|bytes-packed, multiple values should be separated with ','
```
## Batch prove and verify:
```
cargo run --release -- --function <FUNCTION_NAME> --wasm <WASM_BINARY> aggregate-prove [OPTIONS]
cargo run --release -- --function <FUNCTION_NAME> --wasm <WASM_BINARY> aggregate-verify --proof <PROOF_PATH> --instances <AGGREGATE_INSTANCE_PATH>
```

## Generate verify contract:
```
cargo run --release --function <FUNCTION_NAME> --wasm <WASM_BINARY> solidity-aggregate-verifier --proof <PROOF_PATH> --instances <AGGREGATE_INSTANCE_PATH>
```

# Operations Spec [WIP]
We uses z3 (https://github.com/Z3Prover/z3) to check that all operation are compiled to zkp circuits correctly.

[This is a WIP project, only sample code are provided here. Please contact xgao@zoyoe.com for state circuit customization and application integration. 

# Issue tracking:
* chore: non-feature requirements such as CI/CD, building script or work flow enhancement.
* feat: feature need, we could use feat(circuit), feat(lang), feat(CLI) to categorize features
* bug: bug report, we also could use bug(circuit), bug(lang) to categorize bugs
* doc: documents related issues.

# Project Bootstrap:
* C project: There is a project template for compiling C to wasm with limited host functions (foreign circuits). (see https://github.com/DelphinusLab/zkWasm-C)
* Rust project: WIP
* Browser based project: See https://github.com/zkcrossteam/g1024/ for how to utilizing zkWASM in javascript, how to generate proofs using PAAS service and verify it on chain (contact xgao@zoyoe.com for details about PAAS testnet).
