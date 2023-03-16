use crate::runtime::{host::host_env::HostEnv, ExecutionResult};
use anyhow::Result;
use wasmi::RuntimeValue;

use super::test_circuit_with_env;

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
fn build_test() -> Result<(ExecutionResult<RuntimeValue>, i32)> {
    let textual_repr = r#"
    (module
        (type (;0;) (func (param i32) (result i32)))
        (type (;1;) (func (result i32)))
        (func (;0;) (type 0) (param i32) (result i32)
          (local i32)
          local.get 0
          i32.const 2
          i32.ge_u
          if  ;; label = @1
            loop  ;; label = @2
              local.get 0
              i32.const 1
              i32.sub
              call 0
              local.get 1
              i32.add
              local.set 1
              local.get 0
              i32.const 2
              i32.sub
              local.tee 0
              i32.const 1
              i32.gt_u
              br_if 0 (;@2;)
            end
          end
          local.get 0
          local.get 1
          i32.add)
        (func (;1;) (type 1) (result i32)
          i32.const 10
          call 0)
        (memory (;0;) 2 2)
        (export "memory" (memory 0))
        (export "zkmain" (func 1)))
    "#;

    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    let execution_result = test_circuit_with_env(env, wasm, "zkmain", vec![])?;

    Ok((execution_result, 55))
}

mod tests {
    use super::*;
    use crate::{circuits::ZkWasmCircuitBuilder, test::run_test_circuit};
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_fibonacci_mock() {
        let (execution_result, expected_value) = build_test().unwrap();

        assert_eq!(
            execution_result.result.unwrap(),
            RuntimeValue::I32(expected_value)
        );

        run_test_circuit::<Fp>(execution_result, vec![]).unwrap();
    }

    #[test]
    fn test_fibonacci_full() {
        let (execution_result, expected_value) = build_test().unwrap();

        assert_eq!(
            execution_result.result.unwrap(),
            RuntimeValue::I32(expected_value)
        );

        let builder = ZkWasmCircuitBuilder {
            fid_of_entry: execution_result.fid_of_entry,
            tables: execution_result.tables,
        };

        builder.bench(vec![])
    }
}
