use crate::{
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::{
        host::HostEnv, wasmi_interpreter::Execution, ExecutionResult, WasmInterpreter, WasmRuntime,
    },
};
use anyhow::Result;
use wasmi::{ImportsBuilder, RuntimeValue};

/*
   unsigned long long wasm_input(int);

   unsigned long long fib(unsigned long long n)
   {
       if (n <= 1)
           return n;
       return fib(n - 1) + fib(n - 2);
   }

   unsigned long long test() {
       unsigned long long input = wasm_input(1);
       return fib(input);
   }
*/
pub fn build_test() -> Result<(ExecutionResult<RuntimeValue>, Vec<u64>, i32)> {
    let textual_repr = r#"
    (module
        (import "env" "wasm_input" (func $wasm_input (param i32) (result i64)))
        (export "fib" (func $fib))
        (export "test" (func $test))
        (func $fib (; 1 ;) (param $0 i32) (result i32)
         (block $label$0
          (br_if $label$0
           (i32.ne
            (i32.or
             (get_local $0)
             (i32.const 1)
            )
            (i32.const 1)
           )
          )
          (return
           (get_local $0)
          )
         )
         (i32.add
          (call $fib
           (i32.add
            (get_local $0)
            (i32.const -1)
           )
          )
          (call $fib
           (i32.add
            (get_local $0)
            (i32.const -2)
           )
          )
         )
        )
        (func $test (; 2 ;) (result i32)
         (call $fib
          (i32.wrap/i64
           (call $wasm_input
            (i32.const 1)
           )
          )
         )
        )
       )
    "#;

    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");
    let public_inputs = vec![13];

    let compiler = WasmInterpreter::new();
    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);

    let imports = ImportsBuilder::new().with_resolver("env", &env);
    let compiled_module = compiler
        .compile(&wasm, &imports, &env.function_plugin_lookup)
        .unwrap();

    let execution_result = compiled_module.run(&mut env, "test")?;

    Ok((execution_result, public_inputs, 233))
}

mod tests {
    use super::*;
    use crate::test::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_fibonacci() {
        let (execution_result, public_inputs, expected_value) = build_test().unwrap();

        assert_eq!(
            execution_result.result.unwrap(),
            RuntimeValue::I32(expected_value)
        );

        run_test_circuit(
            execution_result.tables,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }
}
