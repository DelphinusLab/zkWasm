use downcast_rs::{impl_downcast, Downcast};

use std::{borrow::BorrowMut, collections::HashMap};

use specs::host_function::{HostFunctionDesc, HostPlugin};
use wasmi::{
    Error, Externals, FuncInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature,
    Trap,
};

struct Function {
    index: usize,
    handler: Box<dyn Fn(&mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
    signature: specs::host_function::Signature,
}

pub(self) trait BuiltInHostFunction {
    const NAME: &'static str;
    const PLUGIN: HostPlugin;

    fn signature() -> specs::host_function::Signature;
    fn handler(args: RuntimeArgs) -> Option<RuntimeValue>;
}

pub trait ForeignContext: Downcast {}
impl_downcast!(ForeignContext);

pub struct HostEnv {
    functions: HashMap<String, Function>,
    contexts: HashMap<String, Box<dyn ForeignContext>>,
    pub function_plugin_lookup: HashMap<usize, HostFunctionDesc>,
    names: Vec<String>,
}

impl HostEnv {
    pub fn new() -> HostEnv {
        HostEnv {
            functions: HashMap::default(),
            contexts: HashMap::default(),
            names: vec![],
            function_plugin_lookup: HashMap::default(),
        }
    }

    fn get_function_by_index(&self, index: usize) -> &Function {
        let name = self
            .names
            .get(index)
            .expect(&format!("env doesn't provide function at index {}", index));

        self.functions.get(name).unwrap()
    }

    fn get_function_with_context_by_index(
        &mut self,
        index: usize,
    ) -> (&mut dyn ForeignContext, &Function) {
        let name = self
            .names
            .get(index)
            .expect(&format!("env doesn't provide function at index {}", index));

        let t = self.contexts.borrow_mut();
        let ctx = t.get_mut(name).unwrap().as_mut();

        (ctx, self.functions.get(name).unwrap())
    }

    pub fn register_function(
        &mut self,
        name: &str,
        op_index_in_plugin: usize,
        context: Box<dyn ForeignContext>,
        signature: specs::host_function::Signature,
        handler: Box<dyn Fn(&mut dyn ForeignContext, RuntimeArgs) -> Option<RuntimeValue>>,
        plugin: HostPlugin,
    ) -> Result<usize, specs::host_function::Error> {
        if self.functions.get(name).is_some() {
            return Err(specs::host_function::Error::DuplicateRegister);
        }

        let index = self.names.len();

        let f = Function {
            index,
            handler,
            signature,
        };

        self.functions.insert(name.to_string(), f);
        self.contexts.insert(name.to_string(), context);
        self.names.push(name.to_string());
        self.function_plugin_lookup.insert(
            index,
            HostFunctionDesc {
                name: name.to_string(),
                op_index_in_plugin,
                plugin,
            },
        );

        Ok(index)
    }

    fn check_signature(&self, index: usize, signature: &Signature) -> bool {
        let sig = self.get_function_by_index(index);
        let signature: specs::host_function::Signature = signature.clone().into();

        sig.signature == signature
    }
}

impl ModuleImportResolver for HostEnv {
    fn resolve_func(
        &self,
        function_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        let index = if let Some(function) = self.functions.get(function_name) {
            function.index
        } else {
            return Err(Error::Instantiation(format!(
                "Export {} not found",
                function_name
            )));
        };

        if !self.check_signature(index, signature) {
            return Err(Error::Instantiation(format!(
                "Export `{}` doesnt match expected type {:?}",
                function_name, signature
            )));
        }

        Ok(FuncInstance::alloc_host(signature.clone(), index))
    }
}

impl Externals for HostEnv {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        let (context, function) = self.get_function_with_context_by_index(index);

        let mut rev_args = Vec::new();
        for i in args.as_ref() {
            rev_args.push(*i);
        }
        rev_args.reverse();
        let args = RuntimeArgs::from(rev_args.as_slice());

        Ok((function.handler)(context, args))
    }
}
