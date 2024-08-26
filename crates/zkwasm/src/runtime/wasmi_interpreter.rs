use anyhow::Result;
use specs::host_function::HostPlugin;
use wasmi::monitor::Monitor;
use wasmi::ImportResolver;
use wasmi::ModuleInstance;
use wasmi::RuntimeValue;

use crate::foreign::context::ContextOutput;

use super::host::host_env::ExecEnv;
use super::host::host_env::HostEnv;
use super::monitor::WasmiMonitor;
use super::CompiledImage;
use super::ExecutionResult;

pub trait Execution<R> {
    fn run(self, monitor: &mut dyn WasmiMonitor, externals: HostEnv) -> Result<ExecutionResult<R>>;
}

impl Execution<RuntimeValue> for CompiledImage<wasmi::NotStartedModuleRef<'_>> {
    fn run(
        self,
        monitor: &mut dyn WasmiMonitor,
        externals: HostEnv,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        let mut exec_env = ExecEnv {
            host_env: externals,
            observer: monitor.expose_observer(),
        };
        let instance = self
            .instance
            .run_start_tracer(&mut exec_env, monitor)
            .unwrap();

        let result = instance.invoke_export_trace(&self.entry, &[], &mut exec_env, monitor)?;

        let host_statics = exec_env
            .host_env
            .external_env
            .get_statics(exec_env.host_env.k);
        let public_inputs_and_outputs = exec_env
            .host_env
            .internal_env
            .get_context_of_plugin(HostPlugin::HostInput)
            .borrow()
            .expose_public_inputs_and_outputs();
        let outputs = exec_env
            .host_env
            .internal_env
            .get_context_of_plugin(HostPlugin::HostInput)
            .borrow()
            .expose_outputs();
        let context_outputs = ContextOutput(
            exec_env
                .host_env
                .internal_env
                .get_context_of_plugin(HostPlugin::Context)
                .borrow()
                .expose_context_outputs(),
        );

        Ok(ExecutionResult {
            result,
            host_statics,
            guest_statics: monitor.expose_observer().borrow().counter,
            public_inputs_and_outputs,
            outputs,
            context_outputs,
        })
    }
}

pub struct WasmiRuntime;

impl WasmiRuntime {
    pub fn compile<'a, I: ImportResolver>(
        monitor: &mut dyn Monitor,
        module: &'a wasmi::Module,
        imports: &I,
        entry: &str,
    ) -> Result<CompiledImage<wasmi::NotStartedModuleRef<'a>>> {
        let instance =
            ModuleInstance::new(module, imports).expect("failed to instantiate wasm module");
        monitor.register_module(instance.loaded_module.module(), &instance.instance, entry)?;

        Ok(CompiledImage {
            entry: entry.to_owned(),
            instance,
        })
    }
}
