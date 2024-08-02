#![deny(warnings)]

pub mod host;
use std::cell::RefCell;
use std::rc::Rc;

use delphinus_zkwasm::foreign::context::runtime::register_context_foreign;
use delphinus_zkwasm::foreign::log_helper::register_log_foreign;
use delphinus_zkwasm::foreign::require_helper::register_require_foreign;
use delphinus_zkwasm::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::plugins::table::FlushHint;
use delphinus_zkwasm::runtime::monitor::plugins::table::FlushStrategy;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::proof::OpType;

#[derive(Serialize, Deserialize, Debug)]
pub struct HostEnvConfig {
    pub ops: Vec<OpType>,
}

impl HostEnvConfig {
    fn register_op(op: &OpType, env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
        match op {
            OpType::BLS381PAIR => host::ecc_helper::bls381::pair::register_blspair_foreign(env),
            OpType::BLS381SUM => host::ecc_helper::bls381::sum::register_blssum_foreign(env),
            OpType::BN256PAIR => host::ecc_helper::bn254::pair::register_bn254pair_foreign(env),
            OpType::BN256SUM => host::ecc_helper::bn254::sum::register_bn254sum_foreign(env),
            OpType::POSEIDONHASH => host::hash_helper::poseidon::register_poseidon_foreign(env),
            OpType::MERKLE => {
                host::merkle_helper::merkle::register_merkle_foreign(env, tree_db.clone());
                host::merkle_helper::datacache::register_datacache_foreign(env, tree_db);
            }
            OpType::JUBJUBSUM => host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(env),
            OpType::KECCAKHASH => host::hash_helper::keccak256::register_keccak_foreign(env),
        }
    }

    fn register_ops(&self, env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
        for op in &self.ops {
            Self::register_op(op, env, tree_db.clone());
        }
    }
}

pub struct StandardHostEnvBuilder {
    ops: Vec<OpType>,
}

impl Default for StandardHostEnvBuilder {
    fn default() -> Self {
        Self {
            ops: vec![
                OpType::POSEIDONHASH,
                OpType::MERKLE,
                OpType::JUBJUBSUM,
                OpType::KECCAKHASH,
                OpType::BN256SUM,
            ],
        }
    }
}

struct StandardHostEnvFlushStrategy {
    ops: HashMap<usize, usize>,
    // avoid to scan ops for every event
    hint: FlushHint,
}

impl Default for StandardHostEnvFlushStrategy {
    fn default() -> Self {
        Self {
            ops: HashMap::new(),
            hint: FlushHint::No,
        }
    }
}

impl FlushStrategy for StandardHostEnvFlushStrategy {
    #[allow(unreachable_code)]
    fn notify(&mut self, op: usize) {
        let count = self.ops.entry(op).or_insert(0);
        *count += 1;

        if todo!("host table is full to accommodate more ops") {
            self.hint = FlushHint::Demand;
        } else if todo!("an ops block is met") {
            self.hint = FlushHint::Suggest;
        } else {
            self.hint = FlushHint::No;
        }
    }

    fn reset(&mut self) {
        self.ops.clear();
        self.hint = FlushHint::No;
    }

    fn hint(&self) -> FlushHint {
        self.hint
    }
}

impl HostEnvBuilder for StandardHostEnvBuilder {
    fn create_env_without_value(&self, k: u32) -> HostEnv {
        let mut env = HostEnv::new(k);
        let host_env_config = HostEnvConfig {
            ops: self.ops.clone(),
        };
        register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![]);
        host::witness_helper::register_witness_foreign(
            &mut env,
            Rc::new(RefCell::new(HashMap::new())),
        );
        host_env_config.register_ops(&mut env, None);

        env.finalize();

        env
    }

    fn create_env(&self, k: u32, arg: ExecutionArg) -> HostEnv {
        let mut env = HostEnv::new(k);
        let host_env_config = HostEnvConfig {
            ops: self.ops.clone(),
        };

        register_wasm_input_foreign(&mut env, arg.public_inputs, arg.private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, arg.context_inputs);
        host::witness_helper::register_witness_foreign(&mut env, arg.indexed_witness);
        host_env_config.register_ops(&mut env, arg.tree_db);

        env.finalize();

        env
    }

    fn create_flush_strategy(&self) -> Box<dyn FlushStrategy> {
        Box::new(StandardHostEnvFlushStrategy::default())
    }
}
