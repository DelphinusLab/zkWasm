use delphinus_zkwasm::runtime::monitor::plugins::table::Command;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::pairing::bn256::Fr as BabyJubjubFq;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use num_traits::Zero;
use std::ops::AddAssign;
use std::ops::Shl;
use zkwasm_host_circuits::circuits::babyjub::AltJubChip;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::host::jubjub;
use zkwasm_host_circuits::host::ForeignInst;
use zkwasm_host_circuits::proof::OpType;

use super::bn_to_field;
use super::field_to_bn;
use crate::PluginFlushStrategy;

pub mod sum;

const LIMBSZ: usize = 64;
const LIMBNB: usize = 4;

pub fn fetch_fq(limbs: &[u64], index: usize) -> BabyJubjubFq {
    let mut bn = BigUint::zero();
    for i in 0..LIMBNB {
        bn.add_assign(BigUint::from_u64(limbs[index * LIMBNB + i]).unwrap() << (i * LIMBSZ))
    }
    bn_to_field(&bn)
}

fn fetch_g1(limbs: &[u64]) -> jubjub::Point {
    jubjub::Point {
        x: fetch_fq(limbs, 0),
        y: fetch_fq(limbs, 1),
    }
}

pub fn babyjubjub_fq_to_limbs(result_limbs: &mut Vec<u64>, f: BabyJubjubFq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..LIMBNB {
        let d: BigUint = BigUint::from(1_u64).shl(LIMBSZ);
        let r = bn.clone() % d.clone();
        let value = if r == BigUint::from(0_u32) {
            0_u64
        } else {
            r.to_u64_digits()[0]
        };
        bn /= d;
        result_limbs.append(&mut vec![value]);
    }
}

pub(crate) struct JubJubFlushStrategy {
    current: usize,
    group: usize,
    maximal_group: usize,
    new_msm: bool,
}

impl JubJubFlushStrategy {
    pub(crate) fn new(k: u32) -> Self {
        Self {
            current: 0,
            group: 0,
            maximal_group: AltJubChip::<Fr>::max_rounds(k as usize),
            new_msm: true,
        }
    }

    fn group_size() -> usize {
        // new + scalar + point + result point
        1 + 4 + 8 + 8
    }
}

impl PluginFlushStrategy for JubJubFlushStrategy {
    fn notify(&mut self, op: &ForeignInst, value: Option<u64>) -> Vec<Command> {
        let op_type = OpType::JUBJUBSUM as usize;

        self.current += 1;

        if *op as usize == ForeignInst::JubjubSumNew as usize {
            let value = value.unwrap();
            assert!(value == 0 || value == 1);

            self.new_msm = value == 1;

            if self.new_msm {
                return vec![Command::Finalize(op_type), Command::Start(op_type)];
            } else {
                return vec![Command::Start(op_type)];
            }
        }

        if self.current == JubJubFlushStrategy::group_size() {
            self.current = 0;
            self.group += 1;

            let mut commands = vec![Command::Commit(op_type, self.new_msm)];

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
