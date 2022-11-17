use crate::{
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
};

use specs::{CompileTable, ExecutionTable};
use std::{fs::File, io::Read, path::PathBuf};
use wasmi::ImportsBuilder;

pub fn build_test() -> (CompileTable, ExecutionTable, Vec<u64>) {
    let public_inputs = vec![3];

    let mut binary = vec![];

    let path = PathBuf::from("wasm/bsearch_64.wasm");
    let mut f = File::open(path).unwrap();
    f.read_to_end(&mut binary).unwrap();

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new(env.function_plugin_lookup.clone());
    let compiled_module = compiler.compile(&binary, &imports).unwrap();
    let _ = compiler
        .run(
            &mut env,
            &compiled_module,
            "bsearch",
            public_inputs.clone(),
            vec![],
        )
        .unwrap();

    (
        compiler.compile_table(),
        compiler.execution_tables(),
        public_inputs,
    )
}

mod tests {
    use super::*;
    use crate::test::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search() {
        let (compile_table, execution_table, public_inputs) = build_test();

        run_test_circuit(
            compile_table,
            execution_table,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }
}
