use downcast_rs::impl_downcast;
use downcast_rs::Downcast;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::host_function::HostFunctionDesc;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;
use wasmi::Signature;

pub mod host_env;

mod external_circuit_plugin;
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
///   fn finalized(&self) {}
/// }
/// ```
pub trait ForeignContext: Downcast {
    fn finalized(&self);
}
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
