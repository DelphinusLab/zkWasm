use anyhow::Result;
use halo2_proofs::arithmetic::MultiMillerLoop;
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::create_proof;
use halo2_proofs::plonk::keygen_vk;
use halo2_proofs::plonk::verify_proof;
use halo2_proofs::plonk::ProvingKey;
use halo2_proofs::plonk::SingleVerifier;
use halo2_proofs::plonk::VerifyingKey;
use halo2_proofs::poly::commitment::Params;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2_proofs::transcript::Blake2bRead;
use halo2_proofs::transcript::Blake2bWrite;
use log::warn;
use rand::rngs::OsRng;
use specs::etable::EventTable;
use specs::jtable::JumpTable;
use specs::slice::Slice;
use std::marker::PhantomData;
use std::sync::Arc;

use wasmi::tracer::Tracer;
use wasmi::ImportsBuilder;
use wasmi::NotStartedModuleRef;
use wasmi::RuntimeValue;

use crate::checksum::CompilationTableWithParams;
use crate::checksum::ImageCheckSum;
use crate::circuits::config::init_zkwasm_runtime;
use crate::circuits::etable::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::image_table::compute_maximal_pages;
use crate::circuits::ZkWasmCircuit;
use crate::loader::err::Error;
use crate::loader::err::PreCheckErr;
#[cfg(feature = "profile")]
use crate::profile::Profiler;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::HostEnvBuilder;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::CompiledImage;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;
use anyhow::anyhow;

use self::slice::Slices;

pub use specs::TraceBackend;

mod err;
pub mod slice;

const ENTRY: &str = "zkmain";

pub struct ExecutionReturn {
    pub context_output: Vec<u64>,
}

pub struct ZkWasmLoader<E: MultiMillerLoop, Arg, EnvBuilder: HostEnvBuilder<Arg = Arg>> {
    pub k: u32,
    entry: String,
    module: wasmi::Module,
    phantom_functions: Vec<String>,
    _mark: PhantomData<(Arg, EnvBuilder, E)>,
}

impl<E: MultiMillerLoop, T, EnvBuilder: HostEnvBuilder<Arg = T>> ZkWasmLoader<E, T, EnvBuilder> {
    #[allow(dead_code)]
    fn precheck(&self) -> Result<()> {
        fn check_zkmain_exists(module: &wasmi::Module) -> Result<()> {
            use parity_wasm::elements::Internal;

            let export = module.module().export_section().unwrap();

            if let Some(entry) = export.entries().iter().find(|entry| entry.field() == ENTRY) {
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

    pub fn compile(
        &self,
        env: &HostEnv,
        dryrun: bool,
        backend: TraceBackend,
    ) -> Result<CompiledImage<NotStartedModuleRef<'_>, Tracer>> {
        let imports = ImportsBuilder::new().with_resolver("env", env);

        WasmInterpreter::compile(
            &self.module,
            &imports,
            &env.function_description_table(),
            self.entry.as_str(),
            dryrun,
            &self.phantom_functions,
            backend,
            self.compute_slice_capability() as u32,
        )
    }

    pub fn circuit_without_witness(
        &self,
        envconfig: EnvBuilder::HostConfig,
        is_last_slice: bool,
    ) -> Result<ZkWasmCircuit<E::Scalar>> {
        let (env, _wasm_runtime_io) = EnvBuilder::create_env_without_value(self.k, envconfig);

        let compiled_module = self.compile(&env, false, TraceBackend::Memory)?;

        Ok(ZkWasmCircuit::new(
            Slice {
                itable: compiled_module.tables.itable.clone(),
                br_table: compiled_module.tables.br_table.clone(),
                elem_table: compiled_module.tables.elem_table.clone(),
                configure_table: compiled_module.tables.configure_table.clone(),
                static_jtable: compiled_module.tables.static_jtable.clone(),
                imtable: compiled_module.tables.imtable.clone(),
                initialization_state: compiled_module.tables.initialization_state.clone(),
                post_imtable: compiled_module.tables.imtable.clone(),
                post_initialization_state: compiled_module.tables.initialization_state.clone(),

                etable: Arc::new(EventTable::default()),
                frame_table: Arc::new(JumpTable::default()),

                is_last_slice,
            },
            self.compute_slice_capability(),
        ))
    }

    /// Create a ZkWasm Loader
    /// Arguments:
    /// - k: the size of circuit
    /// - image: wasm binary
    /// - phantom_functions: regular expressions of phantom function
    pub fn new(k: u32, image: Vec<u8>, phantom_functions: Vec<String>) -> Result<Self> {
        let mut module = wasmi::Module::from_buffer(&image)?;
        if let Ok(parity_module) = module.module().clone().parse_names() {
            module.module = parity_module;
        } else {
            warn!("Failed to parse name section of the wasm binary.");
        }

        let loader = Self {
            k,
            entry: ENTRY.to_string(),
            module,
            phantom_functions,
            _mark: PhantomData,
        };

        #[cfg(not(test))]
        loader.precheck()?;
        loader.init_env()?;

        Ok(loader)
    }

    #[cfg(test)]
    pub(crate) fn set_entry(&mut self, entry: String) {
        self.entry = entry;
    }

    pub fn create_vkey(
        &self,
        params: &Params<E::G1Affine>,
        circuit: &ZkWasmCircuit<E::Scalar>,
    ) -> Result<VerifyingKey<E::G1Affine>> {
        Ok(keygen_vk(&params, circuit).unwrap())
    }

    pub fn checksum<'a>(
        &self,
        params: &'a Params<E::G1Affine>,
        envconfig: EnvBuilder::HostConfig,
    ) -> Result<Vec<E::G1Affine>> {
        let (env, _wasm_runtime_io) = EnvBuilder::create_env_without_value(self.k, envconfig);

        let compiled_module = self.compile(&env, false, TraceBackend::Memory)?;

        let table_with_params = CompilationTableWithParams {
            table: &compiled_module.tables,
            params,
        };

        Ok(table_with_params.checksum(compute_maximal_pages(self.k)))
    }
}

impl<E: MultiMillerLoop, T, EnvBuilder: HostEnvBuilder<Arg = T>> ZkWasmLoader<E, T, EnvBuilder> {
    pub(crate) fn compute_slice_capability(&self) -> usize {
        // FIXME
        ((1 << self.k) - 200) / EVENT_TABLE_ENTRY_ROWS as usize
    }

    pub fn run(
        &self,
        arg: T,
        config: EnvBuilder::HostConfig,
        dryrun: bool,
        backend: TraceBackend,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        let (env, wasm_runtime_io) = EnvBuilder::create_env(self.k, arg, config);
        let compiled_module = self.compile(&env, dryrun, backend)?;
        let result = compiled_module.run(env, dryrun, wasm_runtime_io)?;
        if !dryrun {
            #[cfg(feature = "profile")]
            result.tables.profile_tables();
        }

        Ok(result)
    }

    pub fn slice(&self, execution_result: ExecutionResult<RuntimeValue>) -> Slices<E::Scalar> {
        Slices::new(execution_result.tables, self.compute_slice_capability())
    }

    pub fn mock_test(
        &self,
        circuit: &ZkWasmCircuit<E::Scalar>,
        instances: &Vec<E::Scalar>,
    ) -> Result<()> {
        let prover = MockProver::run(self.k, circuit, vec![instances.clone()])?;
        assert_eq!(prover.verify(), Ok(()));

        Ok(())
    }

    pub fn create_proof(
        &self,
        params: &Params<E::G1Affine>,
        pk: &ProvingKey<E::G1Affine>,
        circuit: &ZkWasmCircuit<E::Scalar>,
        instances: &Vec<E::Scalar>,
    ) -> Result<Vec<u8>> {
        let mut transcript = Blake2bWrite::init(vec![]);

        create_proof(
            params,
            pk,
            std::slice::from_ref(circuit),
            &[&[&instances[..]]],
            OsRng,
            &mut transcript,
        )?;

        Ok(transcript.finalize())
    }

    fn init_env(&self) -> Result<()> {
        init_zkwasm_runtime(self.k);

        Ok(())
    }

    pub fn verify_proof(
        &self,
        params: &Params<E::G1Affine>,
        vkey: &VerifyingKey<E::G1Affine>,
        instances: &Vec<E::Scalar>,
        proof: Vec<u8>,
    ) -> Result<()> {
        let params_verifier: ParamsVerifier<E> = params.verifier(instances.len()).unwrap();
        let strategy = SingleVerifier::new(&params_verifier);

        verify_proof(
            &params_verifier,
            vkey,
            strategy,
            &[&[&instances]],
            &mut Blake2bRead::init(&proof[..]),
        )
        .unwrap();

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use ark_std::end_timer;
//     use ark_std::start_timer;
//     use halo2_proofs::pairing::bn256::Bn256;
//     use halo2_proofs::pairing::bn256::Fr;
//     use halo2_proofs::pairing::bn256::G1Affine;
//     use halo2_proofs::plonk::keygen_pk;
//     use halo2_proofs::poly::commitment::Params;
//     use std::fs::File;
//     use std::io::Cursor;
//     use std::io::Read;
//     use std::path::PathBuf;

//     use crate::circuits::ZkWasmCircuit;
//     use crate::runtime::host::default_env::DefaultHostEnvBuilder;
//     use crate::runtime::host::default_env::ExecutionArg;

//     use super::ZkWasmLoader;

//     impl ZkWasmLoader<Bn256, ExecutionArg, DefaultHostEnvBuilder> {
//         pub(crate) fn bench_test(&self, circuit: ZkWasmCircuit<Fr>, instances: &Vec<Fr>) {
//             fn prepare_param(k: u32) -> Params<G1Affine> {
//                 let path = PathBuf::from(format!("test_param.{}.data", k));

//                 if path.exists() {
//                     let mut fd = File::open(path.as_path()).unwrap();
//                     let mut buf = vec![];

//                     fd.read_to_end(&mut buf).unwrap();
//                     Params::<G1Affine>::read(Cursor::new(buf)).unwrap()
//                 } else {
//                     // Initialize the polynomial commitment parameters
//                     let timer = start_timer!(|| format!("build params with K = {}", k));
//                     let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(k);
//                     end_timer!(timer);

//                     let mut fd = File::create(path.as_path()).unwrap();
//                     params.write(&mut fd).unwrap();

//                     params
//                 }
//             }

//             let params = prepare_param(self.k);
//             let vkey = self.create_vkey(&params, &circuit).unwrap();
//             let pkey = keygen_pk(&params, vkey, &circuit).unwrap();

//             let proof = self
//                 .create_proof(&params, &pkey, &circuit, &instances)
//                 .unwrap();
//             self.verify_proof(&params, pkey.get_vk(), instances, proof)
//                 .unwrap();
//         }
//     }
// }
//
