use specs::types::ValueType;
use wasmi::{RuntimeArgs, RuntimeValue};

use crate::runtime::host::HostEnv;

fn Sigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn Sigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn sigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn sigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn Ch(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn Maj(args: RuntimeArgs) -> Option<RuntimeValue> {
    // FIXME: implement sigma1
    Some(RuntimeValue::I32(0))
}

fn sha256_env() -> HostEnv {
    let mut env = HostEnv::new();

    env.register_function(
        "Sigma1",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Sigma1,
    )
    .unwrap();

    env.register_function(
        "Ch",
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Ch,
    )
    .unwrap();

    env.register_function(
        "Sigma0",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Sigma0,
    )
    .unwrap();

    env.register_function(
        "Maj",
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Maj,
    )
    .unwrap();

    env.register_function(
        "sigma0",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        sigma0,
    )
    .unwrap();

    env.register_function(
        "sigma1",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        sigma1,
    )
    .unwrap();

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use std::{fs::File, io::Read, path::PathBuf};
    use wasmi::ImportsBuilder;

    #[test]
    fn test_sha256() {
        let mut wasm = vec![];

        let path = PathBuf::from("wasm/sha256_extern.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut wasm).unwrap();

        let compiler = WasmInterpreter::new();
        let mut env = sha256_env();

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler.compile(&wasm, &imports).unwrap();
        let execution_log = compiler
            .run(
                &mut env,
                &compiled_module,
                "Hash_Calculate",
                vec![Value::I32(128), Value::I32(256)], // hash 512bit msg using sha256
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
