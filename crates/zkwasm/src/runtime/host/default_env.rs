use std::sync::Arc;
use std::sync::Mutex;

use crate::foreign::context::runtime::register_context_foreign;
use crate::foreign::log_helper::register_log_foreign;
use crate::foreign::require_helper::register_require_foreign;
use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use specs::args::parse_args;

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

impl From<super::Sequence> for ExecutionArg {
    fn from(seq: super::Sequence) -> ExecutionArg {
        let private_inputs = parse_args(seq.private_inputs.iter().map(|s| s.as_str()).collect());
        let public_inputs = parse_args(seq.public_inputs.iter().map(|s| s.as_str()).collect());
        let context_inputs = parse_args(seq.context_input.iter().map(|s| s.as_str()).collect());
        let context_outputs = Arc::new(Mutex::new(vec![]));
        ExecutionArg {
            private_inputs,
            public_inputs,
            context_inputs,
            context_outputs,
        }
    }
}

pub struct DefaultHostEnvBuilder;

impl HostEnvBuilder for DefaultHostEnvBuilder {
    type Arg = ExecutionArg;

    fn create_env_without_value() -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![], Arc::new(Mutex::new(vec![])));
        env.finalize();

        (env, wasm_runtime_io)
    }

    fn create_env(arg: Self::Arg) -> (HostEnv, WasmRuntimeIO) {
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
