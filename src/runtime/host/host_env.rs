use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use specs::host_function::HostFunctionDesc;
use wasmi::Externals;
use wasmi::ModuleImportResolver;
use wasmi::RuntimeArgs;
use wasmi::RuntimeValue;
use wasmi::Trap;

use crate::runtime::host::HostFunctionExecutionEnv;

use super::external_circuit_plugin::ExternalCircuitEnv;
use super::internal_circuit_plugin::InternalCircuitEnv;
use super::HostFunction;

pub struct HostEnv {
    pub internal_env: InternalCircuitEnv,
    pub external_env: ExternalCircuitEnv,

    finalized: Rc<RefCell<bool>>,
    cached_lookup: Option<HashMap<usize, HostFunction>>,
}

impl HostEnv {
    /// Constructs a host environment
    ///
    /// # Examples
    ///
    /// ```
    /// use delphinus_zkwasm::runtime::host::host_env::HostEnv;
    ///
    /// let env = HostEnv::new();
    /// ```
    pub fn new() -> Self {
        let finalized = Rc::new(RefCell::new(false));

        Self {
            internal_env: InternalCircuitEnv::new(finalized.clone()),
            external_env: ExternalCircuitEnv::new(finalized.clone()),
            cached_lookup: None,
            finalized,
        }
    }

    /// Finalizes the host environment to allocate the index of registered objects.
    /// After finalizing the host env, registering any object causes a panic.
    pub fn finalize(&mut self) {
        let mut lookup = HashMap::<usize, HostFunction>::new();

        let mut internal_op_allocator_offset = self.external_env.functions.len();

        for (name, op) in &self.external_env.functions {
            assert!(
                op.op_index < internal_op_allocator_offset,
                "Specify op index too large."
            );

            println!("op_index {}", op.op_index);

            lookup
                .insert(
                    op.op_index,
                    HostFunction {
                        desc: HostFunctionDesc::External {
                            name: name.to_owned(),
                            op: op.op_index,
                            sig: op.sig.into(),
                        },
                        execution_env: HostFunctionExecutionEnv {
                            ctx: op.plugin.ctx.clone(),
                            cb: op.cb.clone(),
                        },
                    },
                )
                .map(|_| panic!("conflicting op index of foreign function"));
        }

        for (name, op) in &mut self.internal_env.functions {
            op.index = Some(internal_op_allocator_offset);

            lookup.insert(
                internal_op_allocator_offset,
                HostFunction {
                    desc: HostFunctionDesc::Internal {
                        name: name.to_owned(),
                        op_index_in_plugin: op.index_within_plugin,
                        plugin: op.plugin,
                    },
                    execution_env: HostFunctionExecutionEnv {
                        ctx: self
                            .internal_env
                            .plugins
                            .get(&op.plugin)
                            .unwrap()
                            .ctx
                            .clone(),
                        cb: op.cb.clone(),
                    },
                },
            );

            internal_op_allocator_offset += 1;
        }

        self.cached_lookup = Some(lookup);

        let mut finalized = self.finalized.borrow_mut();
        *finalized = true;
    }

    pub fn function_description_table(&self) -> HashMap<usize, HostFunctionDesc> {
        assert!(
            *self.finalized.borrow(),
            "HostEnv has not been finalized. Please invoke finalized() first."
        );

        self.cached_lookup
            .clone()
            .unwrap()
            .iter()
            .map(|(idx, host_function)| (*idx, host_function.desc.clone()))
            .collect()
    }
}

impl ModuleImportResolver for HostEnv {
    fn resolve_func(
        &self,
        function_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        self.external_env
            .resolve_func(function_name, signature)
            .or_else(|_| self.internal_env.resolve_func(function_name, signature))
    }
}

impl Externals for HostEnv {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match self.cached_lookup.clone().unwrap().get_mut(&index) {
            Some(function) => {
                let ctx = function.execution_env.ctx.clone();
                let mut ctx = (*ctx).borrow_mut();
                let ctx = ctx.as_mut();

                Ok((function.execution_env.cb)(ctx, args))
            }
            None => unreachable!(),
        }
    }
}
