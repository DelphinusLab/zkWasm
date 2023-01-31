use crate::{
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::{host::HostEnv, wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
};

use anyhow::Result;
use specs::Tables;
use std::{fs::File, io::Read, path::PathBuf};
use wasmi::ImportsBuilder;

fn build_test() -> Result<(Tables, Vec<u64>)> {
    let public_inputs = vec![3];

    let mut binary = vec![];

    let path = PathBuf::from("wasm/bsearch_64.wasm");
    let mut f = File::open(path).unwrap();
    f.read_to_end(&mut binary).unwrap();

    let compiler = WasmInterpreter::new();

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiled_module = compiler
        .compile(&binary, &imports, &env.function_plugin_lookup)
        .unwrap();
    let execution_result = compiled_module.run(&mut env, "bsearch")?;

    Ok((execution_result.tables, public_inputs))
}

mod tests {
    use super::*;
    use crate::{circuits::ZkWasmCircuitBuilder, test::run_test_circuit};
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search_mock() {
        let (tables, public_inputs) = build_test().unwrap();

        run_test_circuit(
            tables,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }

    #[test]
    fn test_binary_search_full() {
        let (execution_result, public_inputs) = build_test().unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
