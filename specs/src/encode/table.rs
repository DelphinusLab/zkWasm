use crate::brtable::{BrTableEntry, ElemEntry, IndirectClass};

use super::FromBn;
use num_bigint::{BigUint, ToBigUint};

lazy_static! {
    static ref INDIRECT_CLASS_SHIFT: BigUint = BigUint::from(1u64) << 192;
}

pub fn encode_br_table_entry<T: FromBn>(
    fid: T,
    iid: T,
    index: T,
    drop: T,
    keep: T,
    dst_pc: T,
) -> T {
    T::from_bn(&(BigUint::from(IndirectClass::BrTable as u64))) * T::from_bn(&INDIRECT_CLASS_SHIFT)
        + fid * T::from_bn(&(BigUint::from(1u64) << 96))
        + iid * T::from_bn(&(BigUint::from(1u64) << 80))
        + index * T::from_bn(&(BigUint::from(1u64) << 64))
        + drop * T::from_bn(&(BigUint::from(1u64) << 48))
        + keep * T::from_bn(&(BigUint::from(1u64) << 32))
        + dst_pc
}

pub fn encode_elem_entry<T: FromBn>(table_idx: T, type_idx: T, offset: T, func_idx: T) -> T {
    T::from_bn(&(BigUint::from(IndirectClass::CallIndirect as u64)))
        * T::from_bn(&INDIRECT_CLASS_SHIFT)
        + table_idx * T::from_bn(&(BigUint::from(1u64) << 96))
        + type_idx * T::from_bn(&(BigUint::from(1u64) << 64))
        + offset * T::from_bn(&(BigUint::from(1u64) << 32))
        + func_idx
}

impl BrTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_br_table_entry(
            BigUint::from(self.fid),
            BigUint::from(self.iid),
            BigUint::from(self.index),
            BigUint::from(self.drop),
            BigUint::from(self.keep),
            BigUint::from(self.dst_pc),
        )
    }
}

impl ElemEntry {
    pub fn encode(&self) -> BigUint {
        encode_elem_entry(
            BigUint::from(self.table_idx),
            BigUint::from(self.type_idx),
            BigUint::from(self.offset),
            BigUint::from(self.func_idx),
        )
    }
}

pub fn encode_frame_table_entry<T: FromBn>(
    frame_id: T,
    last_frame_id: T,
    callee_fid: T,
    fid: T,
    iid: T,
) -> T {
    const EID_SHIFT: usize = 128;
    const LAST_JUMP_EID_SHIFT: usize = 96;
    const CALLEE_FID: usize = 64;
    const FID_SHIFT: usize = 32;

    frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << EID_SHIFT))
        + last_frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << LAST_JUMP_EID_SHIFT))
        + callee_fid * T::from_bn(&(1u64.to_biguint().unwrap() << CALLEE_FID))
        + fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_SHIFT))
        + iid
}
