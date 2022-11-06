use crate::{
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::{host::HostEnv, CompileOutcome, ExecutionOutcome, WasmInterpreter, WasmRuntime},
};

use std::{fs::File, io::Read, path::PathBuf};
use wasmi::{tracer::Tracer, ImportsBuilder, Module, ModuleRef};

pub fn build_test() -> (
    CompileOutcome<Module, ModuleRef, Tracer>,
    ExecutionOutcome,
    Vec<u64>,
) {
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
    let execution_log = compiler
        .run(
            &mut env,
            &compiled_module,
            "bsearch",
            public_inputs.clone(),
            vec![],
        )
        .unwrap();

    (compiled_module, execution_log, public_inputs)
}

mod tests {
    use super::*;
    use crate::test::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search() {
        let (compiled_module, execution_log, public_inputs) = build_test();

        run_test_circuit(
            compiled_module.tables,
            execution_log.tables,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }
}
