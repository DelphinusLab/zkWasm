use anyhow::Result;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::poly::commitment::Params;
use log::warn;

use specs::CompilationTable;

use wasmi::ImportsBuilder;
use wasmi::NotStartedModuleRef;
use wasmi::RuntimeValue;

use crate::checksum::ImageCheckSum;

use crate::loader::err::Error;
use crate::loader::err::PreCheckErr;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::monitor::WasmiMonitor;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::CompiledImage;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;
use anyhow::anyhow;

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

        let mut module = Module::from_buffer(image)?;
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

        Ok(loader)
    }

    #[cfg(test)]
    pub(crate) fn set_entry(&mut self, entry: String) {
        self.entry = entry;
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

    /// Compute the checksum of the compiled wasm image.
    pub fn checksum<C: CurveAffine>(
        &self,
        params: &Params<C>,
        compilation_table: &CompilationTable,
    ) -> Result<Vec<C>> {
        Ok(compilation_table.checksum(self.k, params))
    }
}
