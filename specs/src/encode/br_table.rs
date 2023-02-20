use num_bigint::BigUint;

use super::FromBn;
use crate::{
    brtable::{BrTableEntry, ElemEntry, IndirectClass},
    utils::common_range::COMMON_RANGE_OFFSET,
};

const INDIRECT_CLASS_SHIFT: u32 = 192;

pub fn encode_br_table_entry<T: FromBn>(
    fid: T,
    iid: T,
    index: T,
    drop: T,
    keep: T,
    dst_pc: T,
) -> T {
    const DST_PC_SHIFT: u32 = 0;
    const KEEP_SHIFT: u32 = DST_PC_SHIFT + COMMON_RANGE_OFFSET;
    const DROP_SHIFT: u32 = KEEP_SHIFT + COMMON_RANGE_OFFSET;
    const INDEX_SHIFT: u32 = DROP_SHIFT + COMMON_RANGE_OFFSET;
    const IID_SHIFT: u32 = INDEX_SHIFT + COMMON_RANGE_OFFSET;
    const FID_SHIFT: u32 = IID_SHIFT + COMMON_RANGE_OFFSET;

    assert!(FID_SHIFT + COMMON_RANGE_OFFSET <= INDIRECT_CLASS_SHIFT);

    T::from_bn(&(BigUint::from(IndirectClass::BrTable as u64)))
        * T::from_bn(&(BigUint::from(1u64) << INDIRECT_CLASS_SHIFT))
        + fid * T::from_bn(&(BigUint::from(1u64) << FID_SHIFT))
        + iid * T::from_bn(&(BigUint::from(1u64) << IID_SHIFT))
        + index * T::from_bn(&(BigUint::from(1u64) << INDEX_SHIFT))
        + drop * T::from_bn(&(BigUint::from(1u64) << DROP_SHIFT))
        + keep * T::from_bn(&(BigUint::from(1u64) << KEEP_SHIFT))
        + dst_pc
}

pub fn encode_elem_entry<T: FromBn>(table_idx: T, type_idx: T, offset: T, func_idx: T) -> T {
    const FUNC_INDEX: u32 = 0;
    const OFFSET_SHIFT: u32 = FUNC_INDEX + COMMON_RANGE_OFFSET;
    const TYPE_INDEX_SHIFT: u32 = OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const TABLE_INDEX_SHIFT: u32 = TYPE_INDEX_SHIFT + COMMON_RANGE_OFFSET;

    assert!(TABLE_INDEX_SHIFT + COMMON_RANGE_OFFSET <= INDIRECT_CLASS_SHIFT);

    T::from_bn(&(BigUint::from(IndirectClass::CallIndirect as u64)))
        * T::from_bn(&(BigUint::from(1u64) << INDIRECT_CLASS_SHIFT))
        + table_idx * T::from_bn(&(BigUint::from(1u64) << TABLE_INDEX_SHIFT))
        + type_idx * T::from_bn(&(BigUint::from(1u64) << TYPE_INDEX_SHIFT))
        + offset * T::from_bn(&(BigUint::from(1u64) << OFFSET_SHIFT))
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
