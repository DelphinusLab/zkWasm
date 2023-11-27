pub mod host;
use std::cell::RefCell;
use std::rc::Rc;

use delphinus_zkwasm::foreign::context::runtime::register_context_foreign;
use delphinus_zkwasm::foreign::log_helper::register_log_foreign;
use delphinus_zkwasm::foreign::require_helper::register_require_foreign;
use delphinus_zkwasm::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use delphinus_zkwasm::runtime::wasmi_interpreter::WasmRuntimeIO;

use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ContextOutput;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::host::Sequence;
use specs::args::parse_args;
use std::sync::Arc;
use std::sync::Mutex;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::proof::OpType;
use serde::{Deserialize, Serialize};

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// Context outputs for `wasm_write_context()`
    pub context_outputs: Arc<Mutex<Vec<u64>>>,
    /// db src
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
}

impl ContextOutput for ExecutionArg {
    fn get_context_outputs(&self) -> Arc<Mutex<Vec<u64>>> {
        self.context_outputs.clone()
    }
}

impl From<Sequence> for ExecutionArg {
    fn from(seq: Sequence) -> ExecutionArg {
        let private_inputs = parse_args(seq.private_inputs.iter().map(|s| s.as_str()).collect());
        let public_inputs = parse_args(seq.public_inputs.iter().map(|s| s.as_str()).collect());
        let context_inputs = parse_args(seq.context_input.iter().map(|s| s.as_str()).collect());
        let context_outputs = Arc::new(Mutex::new(vec![]));
        ExecutionArg {
            private_inputs,
            public_inputs,
            context_inputs,
            context_outputs,
            tree_db: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostEnvConfig {
    pub ops: Vec<OpType>
}

impl Default for HostEnvConfig {
    fn default() -> Self {
        HostEnvConfig {
            ops: vec![
                OpType::POSEIDONHASH,
                OpType::MERKLE,
                OpType::JUBJUBSUM,
            ]
        }
    }
}

impl HostEnvConfig {
    fn register_op(op: &OpType, env: &mut HostEnv) {
        match op {
            OpType::BLS381PAIR => host::ecc_helper::bls381::pair::register_blspair_foreign(env),
            OpType::BLS381SUM => host::ecc_helper::bls381::sum::register_blssum_foreign(env),
            OpType::BN256PAIR => host::ecc_helper::bn254::pair::register_bn254pair_foreign(env),
            OpType::BN256SUM => host::ecc_helper::bn254::sum::register_bn254sum_foreign(env),
            OpType::POSEIDONHASH => host::hash_helper::poseidon::register_poseidon_foreign(env),
            OpType::MERKLE => host::merkle_helper::merkle::register_merkle_foreign(env, None),
            OpType::JUBJUBSUM => host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(env),
            OpType::KECCAKHASH => host::hash_helper::keccak256::register_keccak_foreign(env),
        }
    }

    fn register_ops(&self, env: &mut HostEnv) {
        for op in &self.ops {
            Self::register_op(op, env);
        }
    }
}



pub struct StandardHostEnvBuilder;

impl HostEnvBuilder for StandardHostEnvBuilder {
    type Arg = ExecutionArg;
    type HostConfig = HostEnvConfig;

    fn create_env_without_value(envconfig: Self::HostConfig) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![], Arc::new(Mutex::new(vec![])));
        envconfig.register_ops(&mut env);
        host::hash_helper::poseidon::register_poseidon_foreign(&mut env);
        host::merkle_helper::merkle::register_merkle_foreign(&mut env, None);
        host::ecc_helper::bn254::sum::register_bn254sum_foreign(&mut env);
        host::ecc_helper::bn254::pair::register_bn254pair_foreign(&mut env);
        host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(&mut env);
        host::witness_helper::register_witness_foreign(&mut env);
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
        host::hash_helper::poseidon::register_poseidon_foreign(&mut env);
        host::merkle_helper::merkle::register_merkle_foreign(&mut env, arg.tree_db);
        host::ecc_helper::bn254::sum::register_bn254sum_foreign(&mut env);
        host::ecc_helper::bn254::pair::register_bn254pair_foreign(&mut env);
        host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(&mut env);
        host::witness_helper::register_witness_foreign(&mut env);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
