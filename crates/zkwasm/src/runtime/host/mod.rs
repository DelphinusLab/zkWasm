use self::host_env::HostEnv;
use super::wasmi_interpreter::WasmRuntimeIO;
use downcast_rs::impl_downcast;
use downcast_rs::Downcast;
use serde::Deserialize;
use serde::Serialize;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::host_function::HostFunctionDesc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;
use wasmi::Signature;

pub trait ContextOutput {
    fn get_context_outputs(&self) -> Arc<Mutex<Vec<u64>>>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sequence {
    pub private_inputs: Vec<String>,
    pub public_inputs: Vec<String>,
    pub context_input: Vec<String>,
    pub context_output: Option<PathBuf>,
}

pub mod default_env;
pub mod external_circuit_plugin;

pub mod host_env;
mod internal_circuit_plugin;

trait MatchForeignOpSignature {
    fn match_wasmi_signature(&self, signature: &Signature) -> bool;
}

impl MatchForeignOpSignature for ExternalHostCallSignature {
    /// Currently we only support
    /// * function with one argument and without return value
    /// * function with return value and without any arguments
    fn match_wasmi_signature(&self, signature: &Signature) -> bool {
        match self {
            ExternalHostCallSignature::Argument => {
                signature.params().len() == 1
                    && signature.params()[0] == wasmi::ValueType::I64
                    && signature.return_type() == None
            }
            ExternalHostCallSignature::Return => {
                signature.params().len() == 0
                    && signature.return_type() == Some(wasmi::ValueType::I64)
            }
        }
    }
}

/// Context of the plugin.
///
/// # Examples
///
/// ```
/// use delphinus_zkwasm::runtime::host::ForeignContext;
///
/// struct Context {
///   acc: u64,
/// }
/// impl ForeignContext for Context {
/// }
/// ```
pub trait ForeignContext: Downcast {}
impl_downcast!(ForeignContext);

pub struct ForeignPlugin {
    ctx: Rc<RefCell<Box<dyn ForeignContext>>>,
}

#[derive(Clone)]
struct HostFunctionExecutionEnv {
    ctx: Rc<RefCell<Box<dyn ForeignContext>>>,
    cb: Rc<dyn Fn(&mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
}

#[derive(Clone)]
struct HostFunction {
    desc: HostFunctionDesc,
    execution_env: HostFunctionExecutionEnv,
}

/// Implement `HostEnvBuilder` to support customized foreign plugins.
pub trait HostEnvBuilder {
    /// Argument type
    type Arg;
    type HostConfig: Default;
    /// Create an empty env without value, this is used by compiling, computing hash
    fn create_env_without_value(config: Self::HostConfig) -> (HostEnv, WasmRuntimeIO);
    /// Create an env with execution parameters, this is used by dry-run, run
    fn create_env(env: Self::Arg) -> (HostEnv, WasmRuntimeIO);
}
