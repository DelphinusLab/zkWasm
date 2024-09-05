use specs::external_host_call_table::ExternalHostCallSignature;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmi::FuncInstance;
use wasmi::ModuleImportResolver;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;

use crate::runtime::monitor::observer::Observer;

use super::ForeignContext;
use super::ForeignPlugin;
use super::ForeignStatics;
use super::MatchForeignOpSignature;

pub(super) struct ForeignOp {
    pub op_index: usize,
    pub sig: ExternalHostCallSignature,
    pub plugin: Rc<ForeignPlugin>,
    pub cb: Rc<dyn Fn(&Observer, &mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
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
        name: &str,
        ctx: Box<dyn ForeignContext>,
    ) -> Rc<ForeignPlugin> {
        Rc::new(ForeignPlugin {
            name: name.to_string(),
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
        cb: Rc<dyn Fn(&Observer, &mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
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

    pub fn get_statics(&self, k: u32) -> HashMap<String, ForeignStatics> {
        let mut m = HashMap::new();
        for v in self.functions.values() {
            let plugin_name = &v.plugin.name;

            if !m.contains_key(plugin_name) {
                if let Some(stat) = (v.plugin.ctx).as_ref().borrow().get_statics(k) {
                    m.insert(plugin_name.to_string(), stat);
                }
            }
        }
        m
    }
}

impl ModuleImportResolver for ExternalCircuitEnv {
    fn resolve_func(
        &self,
        function_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        if let Some(function) = self.functions.get(function_name) {
            if function.sig.match_wasmi_signature(signature) {
                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    function.op_index,
                ))
            } else {
                Err(wasmi::Error::Instantiation(format!(
                    "Export `{}` doesn't match expected type {:?}",
                    function_name, signature
                )))
            }
        } else {
            Err(wasmi::Error::Instantiation(format!(
                "Export {} not found",
                function_name
            )))
        }
    }
}
