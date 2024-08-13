use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use zkwasm_host_circuits::host::db::TreeDB;

use crate::foreign::context::runtime::register_context_foreign;
use crate::foreign::log_helper::register_log_foreign;
use crate::foreign::require_helper::register_require_foreign;
use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::monitor::plugins::table::Command;
use crate::runtime::monitor::plugins::table::Event;
use crate::runtime::monitor::plugins::table::FlushStrategy;

use super::host_env::HostEnv;
use super::HostEnvBuilder;

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// indexed witness context
    pub indexed_witness: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    /// db src
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
}

pub struct DefaultHostEnvBuilder;

struct DefaultFlushStrategy;

impl FlushStrategy for DefaultFlushStrategy {
    fn notify(&mut self, _event: Event) -> Command {
        Command::Noop
    }
}

impl HostEnvBuilder for DefaultHostEnvBuilder {
    fn create_env_without_value(&self, k: u32) -> HostEnv {
        let mut env = HostEnv::new(k);
        register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![]);
        env.finalize();

        env
    }

    fn create_env(&self, k: u32, arg: ExecutionArg) -> HostEnv {
        let mut env = HostEnv::new(k);
        register_wasm_input_foreign(&mut env, arg.public_inputs, arg.private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, arg.context_inputs);
        env.finalize();

        env
    }

    fn create_flush_strategy(&self, _k: u32) -> Box<dyn FlushStrategy> {
        Box::new(DefaultFlushStrategy)
    }
}
