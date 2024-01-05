use specs::host_function::HostPlugin;
use specs::host_function::Signature;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmi::tracer::Observer;
use wasmi::FuncInstance;
use wasmi::ModuleImportResolver;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;

use super::ForeignContext;
use super::ForeignPlugin;

pub(super) struct ForeignOp {
    pub index: Option<usize>,
    pub index_within_plugin: usize,
    pub sig: Signature,
    pub plugin: HostPlugin,
    pub cb: Rc<dyn Fn(&Observer, &mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
}

pub struct InternalCircuitEnv {
    pub(super) plugins: HashMap<HostPlugin, ForeignPlugin>,
    pub(super) functions: HashMap<String, ForeignOp>,
    finalized: Rc<RefCell<bool>>,
}

impl InternalCircuitEnv {
    pub(super) fn new(finalized: Rc<RefCell<bool>>) -> Self {
        Self {
            plugins: HashMap::new(),
            functions: HashMap::new(),
            finalized,
        }
    }

    pub fn register_plugin(
        &mut self,
        name: &str,
        plugin: HostPlugin,
        context: Box<dyn ForeignContext>,
    ) {
        let _ = self.plugins.insert(
            plugin,
            ForeignPlugin {
                name: name.to_string(),
                ctx: Rc::new(RefCell::new(context)),
            },
        );
    }

    pub fn register_function(
        &mut self,
        function_name: &str,
        sig: Signature,
        plugin: HostPlugin,
        index_within_plugin: usize,
        cb: Rc<dyn Fn(&Observer, &mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
    ) {
        assert!(!*self.finalized.borrow());

        self.functions.insert(
            function_name.to_owned(),
            ForeignOp {
                index: None,
                index_within_plugin,
                sig,
                plugin,
                cb,
            },
        );
    }
}

impl ModuleImportResolver for InternalCircuitEnv {
    fn resolve_func(
        &self,
        function_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        if let Some(ForeignOp { index, sig, .. }) = self.functions.get(function_name) {
            if *sig == signature.clone().into() {
                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    index.expect("Unsolved host function index."),
                ))
            } else {
                Err(wasmi::Error::Instantiation(format!("Signature not match",)))
            }
        } else {
            Err(wasmi::Error::Instantiation(format!(
                "Export {} not found",
                function_name
            )))
        }
    }
}
