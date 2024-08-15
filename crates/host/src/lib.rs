#![deny(warnings)]

pub mod host;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::rc::Rc;

use delphinus_zkwasm::foreign::context::runtime::register_context_foreign;
use delphinus_zkwasm::foreign::log_helper::register_log_foreign;
use delphinus_zkwasm::foreign::require_helper::register_require_foreign;
use delphinus_zkwasm::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::plugins::table::Command;
use delphinus_zkwasm::runtime::monitor::plugins::table::Event;
use delphinus_zkwasm::runtime::monitor::plugins::table::FlushStrategy;
use halo2_proofs::pairing::bn256::Fr;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use zkwasm_host_circuits::circuits::babyjub::AltJubChip;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::merkle::MerkleChip;
use zkwasm_host_circuits::circuits::poseidon::PoseidonChip;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::ForeignInst;
use zkwasm_host_circuits::proof::OpType;

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

#[derive(Default)]
struct StandardHostEnvFlushStrategy {
    k: u32,
    ops: HashMap<usize, (usize, usize)>,
}

fn get_group_size(optype: &OpType) -> usize {
    match optype {
        OpType::MERKLE => 1 + 4 + 4 + 4, // address + set_root + get/set + get_root
        OpType::JUBJUBSUM => 1 + 4 + 8 + 8, // new + scalar + point + result point
        OpType::POSEIDONHASH => 1 + 4 * 8 + 4, // new + push + result
        _ => unreachable!(),
    }
}

fn get_max_bound(optype: &OpType, k: usize) -> usize {
    match optype {
        OpType::MERKLE => MerkleChip::<Fr, MERKLE_TREE_HEIGHT>::max_rounds(k as usize),
        OpType::JUBJUBSUM => AltJubChip::<Fr>::max_rounds(k as usize),
        OpType::POSEIDONHASH => PoseidonChip::max_rounds(k as usize),
        _ => unreachable!(),
    }
}

impl FlushStrategy for StandardHostEnvFlushStrategy {
    fn notify(&mut self, op: Event) -> Command {
        match op {
            Event::HostCall(op) => {
                let op_type = ForeignInst::from_usize(op).unwrap().get_optype();
                if let Some(optype) = op_type {
                    let (count, total) = self.ops.entry(optype.clone() as usize).or_insert((0, 0));
                    let group_size = get_group_size(&optype);

                    *count += 1;

                    if *count == 1 {
                        Command::Start(optype as usize)
                    } else if *count == group_size {
                        *total += 1;
                        *count = 0;

                        if *total >= get_max_bound(&optype, self.k as usize) {
                            Command::CommitAndAbort(optype as usize)
                        } else {
                            Command::Commit(optype as usize)
                        }
                    } else {
                        Command::Noop
                    }
                } else {
                    Command::Noop
                }
            }
            Event::Reset => {
                self.ops.clear();
                Command::Noop
            }
        }
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

    fn create_flush_strategy(&self, k: u32) -> Box<dyn FlushStrategy> {
        Box::new(StandardHostEnvFlushStrategy {
            k,
            ops: HashMap::new(),
        })
    }
}
