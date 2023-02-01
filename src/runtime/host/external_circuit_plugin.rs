use specs::external_host_call_table::ExternalHostCallSignature;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasmi::{FuncInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue};

use super::{ForeignContext, ForeignPlugin, MatchForeignOpSignature};

pub(super) struct ForeignOp {
    pub op_index: usize,
    pub sig: ExternalHostCallSignature,
    pub plugin: Rc<ForeignPlugin>,
    pub cb: Rc<dyn Fn(&mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
}

pub struct ExternalCircuitEnv {
    pub(super) functions: HashMap<String, ForeignOp>,
    finalized: Rc<RefCell<bool>>,
}

impl ExternalCircuitEnv {
    pub(super) fn new(finalized: Rc<RefCell<bool>>) -> Self {
        Self {
            functions: HashMap::new(),
            finalized,
        }
    }

    /// Register a plugin without circuit
    pub fn register_plugin(
        &mut self,
        _name: &str,
        ctx: Box<dyn ForeignContext>,
    ) -> Rc<ForeignPlugin> {
        Rc::new(ForeignPlugin {
            ctx: Rc::new(RefCell::new(ctx)),
        })
    }

    /// Register a foreign function to a registed plugin
    pub fn register_function(
        &mut self,
        name: &str,
        op_index: usize,
        sig: ExternalHostCallSignature,
        plugin: Rc<ForeignPlugin>,
        cb: Rc<dyn Fn(&mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
    ) {
        assert!(!*self.finalized.borrow());

        self.functions.insert(
            name.to_owned(),
            ForeignOp {
                op_index,
                sig,
                plugin,
                cb,
            },
        );
    }
}

impl ModuleImportResolver for ExternalCircuitEnv {
    fn resolve_func(
        &self,
        function_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        for (name, function) in &self.functions {
            if name == function_name {
                if function.sig.match_wasmi_signature(signature) {
                    return Ok(FuncInstance::alloc_host(
                        signature.clone(),
                        function.op_index,
                    ));
                } else {
                    return Err(wasmi::Error::Instantiation(format!(
                        "Export `{}` doesnt match expected type {:?}",
                        function_name, signature
                    )));
                }
            }
        }

        return Err(wasmi::Error::Instantiation(format!(
            "Export {} not found",
            function_name
        )));
    }
}
