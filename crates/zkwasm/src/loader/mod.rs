use anyhow::Result;
use halo2_proofs::arithmetic::CurveAffine;
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

use specs::CompilationTable;

use wasmi::ImportsBuilder;
use wasmi::NotStartedModuleRef;
use wasmi::RuntimeValue;

use crate::checksum::ImageCheckSum;

use crate::circuits::config::init_zkwasm_runtime;
use crate::circuits::ZkWasmCircuit;
use crate::error::BuildingCircuitError;
use crate::loader::err::Error;
use crate::loader::err::PreCheckErr;
#[cfg(feature = "profile")]
use crate::profile::Profiler;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::monitor::WasmiMonitor;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::CompiledImage;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;
use anyhow::anyhow;

use self::slice::Slices;

pub use specs::TraceBackend;
pub use wasmi::Module;

mod err;
pub mod slice;

const ENTRY: &str = "zkmain";

pub struct ExecutionReturn {
    pub context_output: Vec<u64>,
}

pub struct ZkWasmLoader {
    pub k: u32,
    entry: String,
    env: HostEnv,
}

impl ZkWasmLoader {
    pub fn parse_module(image: &Vec<u8>) -> Result<Module> {
        fn precheck(_module: &Module) -> Result<()> {
            #[allow(dead_code)]
            fn check_zkmain_exists(module: &Module) -> Result<()> {
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

            #[cfg(not(test))]
            check_zkmain_exists(_module)?;
            // TODO: check the signature of zkmain function.
            // TODO: check the relation between maximal pages and K.
            // TODO: check the instructions of phantom functions.
            // TODO: check phantom functions exists.
            // TODO: check if instructions are supported.

            Ok(())
        }

        let mut module = Module::from_buffer(&image)?;
        if let Ok(parity_module) = module.module().clone().parse_names() {
            module.module = parity_module;
        } else {
            warn!("Failed to parse name section of the wasm binary.");
        }

        precheck(&module)?;

        Ok(module)
    }
}

impl ZkWasmLoader {
    pub fn compile<'a>(
        &self,
        module: &'a Module,
        monitor: &mut dyn WasmiMonitor,
    ) -> Result<CompiledImage<NotStartedModuleRef<'a>>> {
        let imports = ImportsBuilder::new().with_resolver("env", &self.env);

        WasmInterpreter::compile(monitor, module, &imports, self.entry.as_str())
    }

    pub fn circuit_without_witness<E: MultiMillerLoop>(
        &mut self,
        _is_last_slice: bool,
    ) -> Result<ZkWasmCircuit<E::Scalar>, BuildingCircuitError> {
        todo!()
        /*
        let k = self.k;

        let env = env_builder.create_env_without_value(k);

        let compiled_module = self.compile(&env, false, TraceBackend::Memory).unwrap();

        ZkWasmCircuit::new(
            k,
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
        )
        */
    }

    /// Create a ZkWasm Loader
    ///
    /// Arguments:
    /// - k: the size of circuit
    /// - env: HostEnv for wasmi
    pub fn new(k: u32, env: HostEnv) -> Result<Self> {
        let loader = Self {
            k,
            entry: ENTRY.to_string(),
            env,
        };

        loader.init_env()?;

        Ok(loader)
    }

    #[cfg(test)]
    pub(crate) fn set_entry(&mut self, entry: String) {
        self.entry = entry;
    }

    pub fn create_vkey<E: MultiMillerLoop>(
        &self,
        params: &Params<E::G1Affine>,
        circuit: &ZkWasmCircuit<E::Scalar>,
    ) -> Result<VerifyingKey<E::G1Affine>> {
        Ok(keygen_vk(&params, circuit).unwrap())
    }
}

impl ZkWasmLoader {
    pub fn run(
        self,
        compiled_module: CompiledImage<NotStartedModuleRef<'_>>,
        monitor: &mut dyn WasmiMonitor,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        compiled_module.run(monitor, self.env)
    }

    #[deprecated]
    pub fn slice<E: MultiMillerLoop>(
        &self,
        _execution_result: ExecutionResult<RuntimeValue>,
    ) -> Result<Slices<E::Scalar>, BuildingCircuitError> {
        todo!()
        // Slices::new(self.k, execution_result.tables)
    }

    #[deprecated]
    pub fn mock_test<E: MultiMillerLoop>(
        &self,
        circuit: &ZkWasmCircuit<E::Scalar>,
        instances: &Vec<E::Scalar>,
    ) -> Result<()> {
        let prover = MockProver::run(self.k, circuit, vec![instances.clone()])?;
        assert_eq!(prover.verify(), Ok(()));

        Ok(())
    }

    pub fn create_proof<E: MultiMillerLoop>(
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

    pub fn verify_proof<E: MultiMillerLoop>(
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

    /// Compute the checksum of the compiled wasm image.
    pub fn checksum<C: CurveAffine>(
        &self,
        params: &Params<C>,
        compilation_table: &CompilationTable,
    ) -> Result<Vec<C>> {
        Ok(compilation_table.checksum(self.k, params))
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
