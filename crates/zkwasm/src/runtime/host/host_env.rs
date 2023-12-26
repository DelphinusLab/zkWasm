use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use log::debug;
use specs::host_function::HostFunctionDesc;
use wasmi::tracer::Tracer;
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
    pub(crate) internal_env: InternalCircuitEnv,
    pub external_env: ExternalCircuitEnv,

    finalized: Rc<RefCell<bool>>,
    cached_lookup: Option<HashMap<usize, HostFunction>>,

    /// Profile foreign function time
    time_profile: BTreeMap<String, u128>,
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
            time_profile: BTreeMap::new(),
        }
    }

    /// Finalizes the host environment to allocate the index of registered objects.
    /// After finalizing the host env, registering any object causes a panic.
    pub fn finalize(&mut self) {
        let mut lookup = HashMap::<usize, HostFunction>::new();

        let mut internal_op_allocator_offset = 0;

        for (name, op) in &self.external_env.functions {
            internal_op_allocator_offset = usize::max(internal_op_allocator_offset, op.op_index);

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

        internal_op_allocator_offset += 1;

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

    pub fn display_time_profile(&self) {
        debug!("Execution time(ms) of Foreign Functions:");
        self.time_profile.iter().for_each(|(func, ms)| {
            debug!("{}:\t{}", func, ms);
        })
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

pub struct ExecEnv {
    pub host_env: HostEnv,
    pub tracer: Rc<RefCell<Tracer>>,
}

impl Externals for ExecEnv {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match self
            .host_env
            .cached_lookup
            .as_ref()
            .unwrap()
            .get(&index)
            .clone()
        {
            Some(HostFunction {
                desc,
                execution_env: HostFunctionExecutionEnv { ctx, cb },
            }) => {
                let mut ctx = (*ctx).borrow_mut();
                let ctx = ctx.as_mut();

                let start = Instant::now();
                let r = cb(&self.tracer.borrow().observer, ctx, args);
                let duration = start.elapsed();

                self.host_env
                    .time_profile
                    .entry(desc.name().to_string())
                    .and_modify(|d| *d += duration.as_millis())
                    .or_insert(duration.as_millis());

                Ok(r)
            }
            None => unreachable!(),
        }
    }
}
