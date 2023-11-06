use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use crate::runtime::ExecutionResult;
use anyhow::Result;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::keygen_pk;
use halo2_proofs::plonk::keygen_vk;
use halo2_proofs::plonk::ProvingKey;
use halo2_proofs::poly::commitment::Params;
use wasmi::RuntimeValue;

use super::test_circuit_with_env;

const K: u32 = 18;

fn setup_uniform_verifier() -> Result<(Params<G1Affine>, ProvingKey<G1Affine>)> {
    let textual_repr = r#"
        (module
            (func (export "zkmain"))
        )
        "#;

    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    let execution_result = test_circuit_with_env(env, WasmRuntimeIO::empty(), wasm, "zkmain")?;

    let builder = ZkWasmCircuitBuilder {
        tables: execution_result.tables,
    };

    let circuit: TestCircuit<Fr> = builder.build_circuit();

    let params = Params::<G1Affine>::unsafe_setup::<Bn256>(K);
    let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk, &circuit).expect("keygen_pk should not fail");

    Ok((params, pk))
}

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
        (export "zkmain" (func 1)))
    "#;

    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    let execution_result = test_circuit_with_env(env, WasmRuntimeIO::empty(), wasm, "zkmain")?;

    Ok((execution_result, 55))
}

mod tests {
    use halo2_proofs::plonk::create_proof;
    use halo2_proofs::plonk::verify_proof;
    use halo2_proofs::plonk::SingleVerifier;
    use halo2_proofs::poly::commitment::ParamsVerifier;
    use halo2_proofs::transcript::Blake2bRead;
    use halo2_proofs::transcript::Blake2bWrite;
    use halo2_proofs::transcript::Challenge255;
    use rand::rngs::OsRng;

    use super::*;
    use crate::circuits::ZkWasmCircuitBuilder;

    #[test]
    fn test_uniform_verifier() {
        let (params, uniform_verifier_pk) = setup_uniform_verifier().unwrap();

        let (execution_result, expected_value) = build_test().unwrap();

        assert_eq!(
            execution_result.result.unwrap(),
            RuntimeValue::I32(expected_value)
        );

        let instances = vec![];

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
        };

        let proof = {
            let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);

            create_proof(
                &params,
                &uniform_verifier_pk,
                &[builder.build_circuit()],
                &[&[&instances]],
                OsRng,
                &mut transcript,
            )
            .expect("proof generation should not fail");

            transcript.finalize()
        };

        {
            let public_inputs_size = 1;

            let params_verifier: ParamsVerifier<Bn256> =
                params.verifier(public_inputs_size).unwrap();

            let strategy = SingleVerifier::new(&params_verifier);
            let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

            verify_proof(
                &params_verifier,
                uniform_verifier_pk.get_vk(),
                strategy,
                &[&[&instances]],
                &mut transcript,
            )
            .unwrap();
        }
    }
}
