use crate::brtable::{BrTableEntry, ElemEntry, IndirectClass};

use super::FromBn;
use num_bigint::BigUint;

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
