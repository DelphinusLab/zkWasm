#![deny(warnings)]

use delphinus_zkwasm::foreign::context::runtime::register_context_foreign;
use delphinus_zkwasm::foreign::log_helper::register_log_foreign;
use delphinus_zkwasm::foreign::require_helper::register_require_foreign;
use delphinus_zkwasm::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::plugins::table::transaction::TransactionId;
use delphinus_zkwasm::runtime::monitor::plugins::table::Command;
use delphinus_zkwasm::runtime::monitor::plugins::table::Event;
use delphinus_zkwasm::runtime::monitor::plugins::table::FlushStrategy;
use host::ecc_helper::jubjub::JubJubFlushStrategy;
use host::hash_helper::poseidon::PoseidonFlushStrategy;
use host::merkle_helper::MerkleFlushStrategy;
use num_traits::FromPrimitive;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::ForeignInst;
use zkwasm_host_circuits::proof::OpType;

pub mod host;

// TODO: move into zkwasm-host-circuits repo

#[derive(Serialize, Deserialize, Debug)]
pub struct HostEnvConfig {
    pub ops: Vec<OpType>,
}

pub const MERKLE_TREE_HEIGHT: usize = 32;

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
    k: u32,
    ops: Vec<OpType>,
}

trait GroupedForeign {
    fn get_optype(&self) -> Option<OpType>;
}

impl GroupedForeign for ForeignInst {
    fn get_optype(&self) -> Option<OpType> {
        match self {
            ForeignInst::MerkleSet => Some(OpType::MERKLE),
            ForeignInst::MerkleGet => Some(OpType::MERKLE),
            ForeignInst::MerkleSetRoot => Some(OpType::MERKLE),
            ForeignInst::MerkleGetRoot => Some(OpType::MERKLE),
            ForeignInst::MerkleAddress => Some(OpType::MERKLE),

            ForeignInst::PoseidonPush => Some(OpType::POSEIDONHASH),
            ForeignInst::PoseidonNew => Some(OpType::POSEIDONHASH),
            ForeignInst::PoseidonFinalize => Some(OpType::POSEIDONHASH),

            ForeignInst::JubjubSumNew => Some(OpType::JUBJUBSUM),
            ForeignInst::JubjubSumPush => Some(OpType::JUBJUBSUM),
            ForeignInst::JubjubSumResult => Some(OpType::JUBJUBSUM),

            _ => None,
        }
    }
}

trait GroupedForeignPlugin {
    fn new_plugin_flush_strategy(&self, k: u32) -> Box<dyn PluginFlushStrategy>;
}

impl GroupedForeignPlugin for OpType {
    fn new_plugin_flush_strategy(&self, k: u32) -> Box<dyn PluginFlushStrategy> {
        match self {
            OpType::POSEIDONHASH => Box::new(PoseidonFlushStrategy::new(k)),
            OpType::MERKLE => Box::new(MerkleFlushStrategy::new(k)),
            OpType::JUBJUBSUM => Box::new(JubJubFlushStrategy::new(k)),
            _ => Box::new(TrivialPluginFlushStrategy {}),
        }
    }
}

trait PluginFlushStrategy {
    fn notify(&mut self, op: &ForeignInst, value: Option<u64>) -> Vec<Command>;
    fn reset(&mut self);

    fn maximal_group(&self) -> Option<usize>;
}

struct TrivialPluginFlushStrategy {}

impl PluginFlushStrategy for TrivialPluginFlushStrategy {
    fn notify(&mut self, _op: &ForeignInst, _value: Option<u64>) -> Vec<Command> {
        vec![Command::Noop]
    }

    fn reset(&mut self) {}

    fn maximal_group(&self) -> Option<usize> {
        None
    }
}

impl StandardHostEnvBuilder {
    pub fn new(k: u32) -> Self {
        Self {
            k,
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

#[derive(Default)]
struct StandardHostEnvFlushStrategy {
    k: u32,
    ops: HashMap<usize, Box<dyn PluginFlushStrategy>>,
}

impl FlushStrategy for StandardHostEnvFlushStrategy {
    fn notify(&mut self, op: Event) -> Vec<Command> {
        match op {
            Event::HostCall(op, value) => {
                let inst = ForeignInst::from_usize(op);
                if inst.is_none() {
                    return vec![Command::Noop];
                }

                let inst = inst.unwrap();
                let op_type = inst.get_optype();
                if op_type.is_none() {
                    return vec![Command::Noop];
                }

                let op_type = op_type.unwrap();
                let plugin = self
                    .ops
                    .entry(op_type.clone() as usize)
                    .or_insert_with(|| op_type.new_plugin_flush_strategy(self.k));

                plugin.notify(&inst, value)
            }
            Event::Reset() => {
                for (_, plugin) in self.ops.iter_mut() {
                    plugin.reset();
                }

                vec![Command::Noop]
            }
        }
    }

    fn maximal_group(&self, transaction: TransactionId) -> Option<usize> {
        // FIXME: add usize to zkwasm-host-circuits repo
        fn optype_from_usize(index: usize) -> OpType {
            match index {
                0 => OpType::BLS381PAIR,
                1 => OpType::BLS381SUM,
                2 => OpType::BN256PAIR,
                3 => OpType::BN256SUM,
                4 => OpType::POSEIDONHASH,
                5 => OpType::KECCAKHASH,
                6 => OpType::MERKLE,
                7 => OpType::JUBJUBSUM,
                _ => unreachable!(),
            }
        }
        optype_from_usize(transaction)
            .new_plugin_flush_strategy(self.k)
            .maximal_group()
    }
}

impl HostEnvBuilder for StandardHostEnvBuilder {
    fn create_env_without_value(&self) -> HostEnv {
        let mut env = HostEnv::new(self.k);
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

    fn create_env(&self, arg: ExecutionArg) -> HostEnv {
        let mut env = HostEnv::new(self.k);
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
        Box::new(StandardHostEnvFlushStrategy {
            k: self.k,
            ops: HashMap::new(),
        })
    }
}
