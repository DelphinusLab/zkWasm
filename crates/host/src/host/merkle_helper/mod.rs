use delphinus_zkwasm::runtime::monitor::plugins::table::Command;
use halo2_proofs::pairing::bn256::Fr;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::merkle::MerkleChip;
use zkwasm_host_circuits::host::ForeignInst;
use zkwasm_host_circuits::proof::OpType;

use crate::PluginFlushStrategy;
use crate::MERKLE_TREE_HEIGHT;

pub mod datacache;
pub mod merkle;

pub(crate) struct MerkleFlushStrategy {
    current: usize,
    group: usize,
    maximal_group: usize,
    is_set: bool,
}

impl MerkleFlushStrategy {
    pub(crate) fn new(k: u32) -> Self {
        Self {
            current: 0,
            group: 0,
            maximal_group: MerkleChip::<Fr, MERKLE_TREE_HEIGHT>::max_rounds(k as usize),
            is_set: false,
        }
    }

    fn group_size() -> usize {
        // address + set_root + get/set + get_root
        1 + 4 + 4 + 4
    }
}

impl PluginFlushStrategy for MerkleFlushStrategy {
    fn notify(&mut self, op: &ForeignInst, _value: Option<u64>) -> Vec<Command> {
        let op_type = OpType::MERKLE as usize;

        self.current += 1;

        if *op as usize == ForeignInst::MerkleAddress as usize {
            self.is_set = false;

            return vec![Command::Start(op_type)];
        }

        if *op as usize == ForeignInst::MerkleSet as usize {
            self.is_set = true;
        }

        if *op as usize == ForeignInst::MerkleGet as usize {
            return vec![Command::Finalize(op_type), Command::Noop];
        }

        if self.current == MerkleFlushStrategy::group_size() {
            self.current = 0;
            self.group += 1;

            let mut commands = if self.is_set {
                vec![Command::Commit(op_type, false), Command::Finalize(op_type)]
            } else {
                vec![Command::Commit(op_type, true)]
            };

            if self.group >= self.maximal_group {
                commands.push(Command::Abort);
            }

            return commands;
        }

        vec![Command::Noop]
    }

    fn reset(&mut self) {
        self.current = 0;
        self.group = 0;
    }

    fn maximal_group(&self) -> Option<usize> {
        Some(self.maximal_group)
    }
}
