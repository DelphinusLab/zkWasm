use crate::foreign::context::ContextOutput;

use self::default_env::ExecutionArg;
use self::host_env::HostEnv;
use downcast_rs::impl_downcast;
use downcast_rs::Downcast;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::host_function::HostFunctionDesc;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;
use wasmi::Signature;

use super::monitor::observer::Observer;

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
                    && signature.return_type().is_none()
            }
            ExternalHostCallSignature::Return => {
                signature.params().is_empty()
                    && signature.return_type() == Some(wasmi::ValueType::I64)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ForeignStatics {
    pub used_round: usize,
    pub max_round: usize,
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
pub trait ForeignContext: Downcast {
    fn get_statics(&self) -> Option<ForeignStatics> {
        None
    }

    fn expose_public_inputs_and_outputs(&self) -> Vec<u64> {
        unreachable!()
    }

    fn expose_outputs(&self) -> Vec<u64> {
        unreachable!()
    }

    fn expose_context_outputs(&self) -> Vec<u64> {
        unreachable!()
    }
}
impl_downcast!(ForeignContext);

pub struct ForeignPlugin {
    pub name: String,
    ctx: Rc<RefCell<Box<dyn ForeignContext>>>,
}

#[derive(Clone)]
struct HostFunctionExecutionEnv {
    ctx: Rc<RefCell<Box<dyn ForeignContext>>>,
    cb: Rc<dyn Fn(&Observer, &mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
}

#[derive(Clone)]
struct HostFunction {
    desc: HostFunctionDesc,
    execution_env: HostFunctionExecutionEnv,
}

pub trait HostEnvArg {
    fn get_context_output(&self) -> ContextOutput;
}

/// Implement `HostEnvBuilder` to support customized foreign plugins.
pub trait HostEnvBuilder {
    /// Create an empty env without value, this is used by compiling, computing hash
    fn create_env_without_value(&self, k: u32) -> HostEnv;
    /// Create an env with execution parameters, this is used by dry-run, run
    fn create_env(&self, k: u32, env: ExecutionArg) -> HostEnv;
}
