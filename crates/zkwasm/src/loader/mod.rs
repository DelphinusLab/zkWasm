use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use anyhow::Result;
use halo2_proofs::arithmetic::MultiMillerLoop;
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::get_advice_commitments_from_transcript;
use halo2_proofs::plonk::keygen_vk;
use halo2_proofs::plonk::verify_proof;
use halo2_proofs::plonk::SingleVerifier;
use halo2_proofs::plonk::VerifyingKey;
use halo2_proofs::poly::commitment::Params;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2aggregator_s::circuits::utils::load_or_create_proof;
use halo2aggregator_s::circuits::utils::TranscriptHash;
use halo2aggregator_s::transcript::poseidon::PoseidonRead;
use specs::ExecutionTable;
use specs::Tables;
use wasmi::tracer::Tracer;
use wasmi::ImportsBuilder;
use wasmi::NotStartedModuleRef;
use wasmi::RuntimeValue;

use crate::checksum::CompilationTableWithParams;
use crate::checksum::ImageCheckSum;
use crate::circuits::config::init_zkwasm_runtime;
use crate::circuits::config::set_zkwasm_k;
use crate::circuits::image_table::IMAGE_COL_NAME;
use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
use crate::loader::err::Error;
use crate::loader::err::PreCheckErr;
use crate::profile::Profiler;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::CompiledImage;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;
use anyhow::anyhow;

mod err;

const ENTRY: &str = "zkmain";

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// Context outputs for `wasm_write_context()`
    pub context_outputs: Rc<RefCell<Vec<u64>>>,
}

pub struct ExecutionReturn {
    pub context_output: Vec<u64>,
}

pub struct ZkWasmLoader<E: MultiMillerLoop> {
    k: u32,
    module: wasmi::Module,
    phantom_functions: Vec<String>,
    _data: PhantomData<E>,
}

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    fn precheck(&self) -> Result<()> {
        fn check_zkmain_exists(module: &wasmi::Module) -> Result<()> {
            use parity_wasm::elements::Internal;

            let export = module.module().export_section().unwrap();

            if let Some(entry) = export
                .entries()
                .iter()
                .find(|entry| entry.field() == "zkmain")
            {
                match entry.internal() {
                    Internal::Function(_fid) => Ok(()),
                    _ => Err(anyhow!(Error::PreCheck(PreCheckErr::ZkmainIsNotFunction))),
                }
            } else {
                Err(anyhow!(Error::PreCheck(PreCheckErr::ZkmainNotExists)))
            }
        }

        check_zkmain_exists(&self.module)?;
        // TODO: check the signature of zkmain function.
        // TODO: check the relation between maximal pages and K.
        // TODO: check the instructions of phantom functions.
        // TODO: check phantom functions exists.
        // TODO: check if instructions are supported.

        Ok(())
    }

    fn compile(&self, env: &HostEnv) -> Result<CompiledImage<NotStartedModuleRef<'_>, Tracer>> {
        let imports = ImportsBuilder::new().with_resolver("env", env);

        WasmInterpreter::compile(
            &self.module,
            &imports,
            &env.function_description_table(),
            ENTRY,
            &self.phantom_functions,
        )
    }

    fn circuit_without_witness(&self) -> Result<TestCircuit<E::Scalar>> {
        let (env, wasm_runtime_io) = HostEnv::new_with_full_foreign_plugins(
            vec![],
            vec![],
            vec![],
            Rc::new(RefCell::new(vec![])),
        );

        let compiled_module = self.compile(&env)?;

        let builder = ZkWasmCircuitBuilder {
            tables: Tables {
                compilation_tables: compiled_module.tables,
                execution_tables: ExecutionTable::default(),
            },
            public_inputs_and_outputs: wasm_runtime_io.public_inputs_and_outputs.borrow().clone(),
        };

        Ok(builder.build_circuit::<E::Scalar>())
    }

    pub fn new(k: u32, image: Vec<u8>, phantom_functions: Vec<String>) -> Result<Self> {
        set_zkwasm_k(k);

        let module = wasmi::Module::from_buffer(&image)?;

        let loader = Self {
            k,
            module,
            phantom_functions,
            _data: PhantomData,
        };

        loader.precheck()?;
        loader.init_env()?;

        Ok(loader)
    }

    pub fn create_vkey(&self, params: &Params<E::G1Affine>) -> Result<VerifyingKey<E::G1Affine>> {
        let circuit = self.circuit_without_witness()?;

        Ok(keygen_vk(&params, &circuit).unwrap())
    }

    pub fn checksum(&self, params: &Params<E::G1Affine>) -> Result<Vec<E::G1Affine>> {
        let (env, _) = HostEnv::new_with_full_foreign_plugins(
            vec![],
            vec![],
            vec![],
            Rc::new(RefCell::new(vec![])),
        );
        let compiled = self.compile(&env)?;

        let table_with_params = CompilationTableWithParams {
            table: &compiled.tables,
            params,
        };

        Ok(table_with_params.checksum())
    }
}

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    pub fn dry_run(&self, arg: ExecutionArg) -> Result<Option<RuntimeValue>> {
        let (mut env, _) = HostEnv::new_with_full_foreign_plugins(
            arg.public_inputs,
            arg.private_inputs,
            arg.context_inputs,
            arg.context_outputs,
        );

        let compiled_module = self.compile(&env)?;

        compiled_module.dry_run(&mut env)
    }

    pub fn run(&self, arg: ExecutionArg) -> Result<ExecutionResult<RuntimeValue>> {
        let (mut env, wasm_runtime_io) = HostEnv::new_with_full_foreign_plugins(
            arg.public_inputs,
            arg.private_inputs,
            arg.context_inputs,
            arg.context_outputs,
        );

        let compiled_module = self.compile(&env)?;

        let result = compiled_module.run(&mut env, wasm_runtime_io)?;

        result.tables.profile_tables();
        result.tables.write_json(None);

        Ok(result)
    }

    pub fn circuit_with_witness(
        &self,
        arg: ExecutionArg,
    ) -> Result<(TestCircuit<E::Scalar>, Vec<E::Scalar>)> {
        let execution_result = self.run(arg)?;

        let instance: Vec<E::Scalar> = execution_result
            .public_inputs_and_outputs
            .clone()
            .iter()
            .map(|v| (*v).into())
            .collect();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
            public_inputs_and_outputs: execution_result.public_inputs_and_outputs,
        };

        println!("output:");
        println!("{:?}", execution_result.outputs);

        Ok((builder.build_circuit(), instance))
    }

    pub fn mock_test(
        &self,
        circuit: &TestCircuit<E::Scalar>,
        instances: &Vec<E::Scalar>,
    ) -> Result<()> {
        let prover = MockProver::run(self.k, circuit, vec![instances.clone()])?;
        assert_eq!(prover.verify(), Ok(()));

        Ok(())
    }

    pub fn create_proof(
        &self,
        params: &Params<E::G1Affine>,
        vkey: VerifyingKey<E::G1Affine>,
        circuit: TestCircuit<E::Scalar>,
        instances: &Vec<E::Scalar>,
    ) -> Result<Vec<u8>> {
        Ok(load_or_create_proof::<E, _>(
            &params,
            vkey,
            circuit,
            &[instances],
            None,
            TranscriptHash::Poseidon,
            false,
        ))
    }

    pub fn init_env(&self) -> Result<()> {
        init_zkwasm_runtime(self.k);

        Ok(())
    }

    pub fn verify_proof(
        &self,
        params: &Params<E::G1Affine>,
        vkey: VerifyingKey<E::G1Affine>,
        instances: Vec<E::Scalar>,
        proof: Vec<u8>,
    ) -> Result<()> {
        let params_verifier: ParamsVerifier<E> = params.verifier(instances.len()).unwrap();
        let strategy = SingleVerifier::new(&params_verifier);

        verify_proof(
            &params_verifier,
            &vkey,
            strategy,
            &[&[&instances]],
            &mut PoseidonRead::init(&proof[..]),
        )
        .unwrap();

        {
            let img_col_idx = vkey
                .cs
                .named_advices
                .iter()
                .find(|(k, _)| k == IMAGE_COL_NAME)
                .unwrap()
                .1;
            let img_col_commitment: Vec<E::G1Affine> =
                get_advice_commitments_from_transcript::<E, _, _>(
                    &vkey,
                    &mut PoseidonRead::init(&proof[..]),
                )
                .unwrap();
            let checksum = self.checksum(params)?;

            assert!(vec![img_col_commitment[img_col_idx as usize]] == checksum)
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ark_std::end_timer;
    use ark_std::start_timer;
    use halo2_proofs::pairing::bn256::Bn256;
    use halo2_proofs::pairing::bn256::Fr;
    use halo2_proofs::pairing::bn256::G1Affine;
    use halo2_proofs::poly::commitment::Params;
    use std::fs::File;
    use std::io::Cursor;
    use std::io::Read;
    use std::path::PathBuf;

    use crate::circuits::TestCircuit;

    use super::ZkWasmLoader;

    impl ZkWasmLoader<Bn256> {
        pub(crate) fn bench_test(&self, circuit: TestCircuit<Fr>, instances: Vec<Fr>) {
            fn prepare_param(k: u32) -> Params<G1Affine> {
                let path = PathBuf::from(format!("test_param.{}.data", k));

                if path.exists() {
                    let mut fd = File::open(path.as_path()).unwrap();
                    let mut buf = vec![];

                    fd.read_to_end(&mut buf).unwrap();
                    Params::<G1Affine>::read(Cursor::new(buf)).unwrap()
                } else {
                    // Initialize the polynomial commitment parameters
                    let timer = start_timer!(|| format!("build params with K = {}", k));
                    let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(k);
                    end_timer!(timer);

                    let mut fd = File::create(path.as_path()).unwrap();
                    params.write(&mut fd).unwrap();

                    params
                }
            }

            let params = prepare_param(self.k);
            let vkey = self.create_vkey(&params).unwrap();

            let proof = self
                .create_proof(&params, vkey.clone(), circuit, &instances)
                .unwrap();
            self.verify_proof(&params, vkey, instances, proof).unwrap();
        }
    }
}
