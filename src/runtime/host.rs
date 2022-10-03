use std::collections::HashMap;

use specs::itable::HostPlugin;
use wasmi::{
    Error, Externals, FuncInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature,
    Trap,
};

struct Function {
    index: usize,
    handler: fn(RuntimeArgs) -> Option<RuntimeValue>,
    signature: specs::host_function::Signature,
}

pub(self) trait BuiltInHostFunction {
    const NAME: &'static str;
    const PLUGIN: HostPlugin;

    fn signature() -> specs::host_function::Signature;
    fn handler(args: RuntimeArgs) -> Option<RuntimeValue>;
}

mod wasm_input {
    use super::BuiltInHostFunction;
    use specs::host_function::Signature;
    use specs::itable::HostPlugin;
    use specs::types::ValueType;

    pub(super) struct Function;

    impl BuiltInHostFunction for Function {
        const NAME: &'static str = "wasm_input";
        const PLUGIN: HostPlugin = HostPlugin::HostInput;

        fn signature() -> Signature {
            Signature {
                params: vec![ValueType::I32],
                return_type: Some(ValueType::I64),
            }
        }

        fn handler(_args: wasmi::RuntimeArgs) -> Option<wasmi::RuntimeValue> {
            Some(wasmi::RuntimeValue::I64(0))
        }
    }
}

pub struct HostEnv {
    functions: HashMap<String, Function>,
    pub function_plugin_lookup: HashMap<usize, HostPlugin>,
    names: Vec<String>,
}

impl HostEnv {
    pub fn new() -> HostEnv {
        let mut env = HostEnv {
            functions: HashMap::default(),
            names: vec![],
            function_plugin_lookup: HashMap::default(),
        };

        macro_rules! register_builtin {
            ($n:ident) => {
                env.register_function(
                    $n::Function::NAME,
                    $n::Function::signature(),
                    $n::Function::handler,
                    $n::Function::PLUGIN,
                )
                .unwrap();
            };
        }

        register_builtin!(wasm_input);

        env
    }

    fn get_function_by_index(&self, index: usize) -> &Function {
        let name = self
            .names
            .get(index)
            .expect(&format!("env doesn't provide function at index {}", index));

        self.functions.get(name).unwrap()
    }

    pub fn register_function(
        &mut self,
        name: &str,
        signature: specs::host_function::Signature,
        handler: fn(RuntimeArgs) -> Option<RuntimeValue>,
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
        self.names.push(name.to_string());
        self.function_plugin_lookup.insert(index, plugin);

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
        let function = self.get_function_by_index(index);

        Ok((function.handler)(args))
    }
}
