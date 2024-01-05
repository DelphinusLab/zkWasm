use std::sync::Arc;
use std::sync::Mutex;

use crate::foreign::context::runtime::register_context_foreign;
use crate::foreign::log_helper::register_log_foreign;
use crate::foreign::require_helper::register_require_foreign;
use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;

use super::host_env::HostEnv;
use super::HostEnvBuilder;

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// Context outputs for `wasm_write_context()`
    pub context_outputs: Arc<Mutex<Vec<u64>>>,
}

impl super::ContextOutput for ExecutionArg {
    fn get_context_outputs(&self) -> Arc<Mutex<Vec<u64>>> {
        self.context_outputs.clone()
    }
}

pub struct DefaultHostEnvBuilder;

impl HostEnvBuilder for DefaultHostEnvBuilder {
    type Arg = ExecutionArg;
    type HostConfig = ();

    fn create_env_without_value(_config: Self::HostConfig) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![], Arc::new(Mutex::new(vec![])));
        env.finalize();

        (env, wasm_runtime_io)
    }

    fn create_env(arg: Self::Arg, _config: Self::HostConfig) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io =
            register_wasm_input_foreign(&mut env, arg.public_inputs, arg.private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, arg.context_inputs, arg.context_outputs);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
