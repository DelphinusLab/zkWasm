use num_bigint::BigUint;
use static_assertions::const_assert;

use crate::brtable::BrTableEntry;
use crate::brtable::ElemEntry;
use crate::brtable::IndirectClass;
use crate::encode::instruction_table::FID_BITS;
use crate::encode::instruction_table::IID_BITS;
use crate::encode::COMMON_RANGE_BITS;

use super::FromBn;

const INDIRECT_CLASS_SHIFT: u32 = 192;
lazy_static! {
    static ref INDIRECT_CLASS_SHIFT_BN: BigUint = BigUint::from(1u64) << 192;
}

pub const BR_TABLE_ENCODE_BOUNDARY: u32 = 224;
// Tag only include 1 bit(BrTable or Elem)
const_assert!(INDIRECT_CLASS_SHIFT < BR_TABLE_ENCODE_BOUNDARY);

pub fn encode_br_table_entry<T: FromBn>(
    fid: T,
    iid: T,
    index: T,
    drop: T,
    keep: T,
    dst_pc: T,
) -> T {
    const FID_SHIFT: u32 = IID_SHIFT + IID_BITS;
    const IID_SHIFT: u32 = INDEX_SHIFT + COMMON_RANGE_BITS;
    const INDEX_SHIFT: u32 = DROP_SHIFT + COMMON_RANGE_BITS;
    const DROP_SHIFT: u32 = KEEP_SHIFT + COMMON_RANGE_BITS;
    const KEEP_SHIFT: u32 = DST_PC_SHIFT + COMMON_RANGE_BITS;
    const DST_PC_SHIFT: u32 = 0;

    assert!(FID_SHIFT + FID_BITS <= INDIRECT_CLASS_SHIFT);

    T::from_bn(&(BigUint::from(IndirectClass::BrTable as u64)))
        * T::from_bn(&INDIRECT_CLASS_SHIFT_BN)
        + fid * T::from_bn(&(BigUint::from(1u64) << FID_SHIFT))
        + iid * T::from_bn(&(BigUint::from(1u64) << IID_SHIFT))
        + index * T::from_bn(&(BigUint::from(1u64) << INDEX_SHIFT))
        + drop * T::from_bn(&(BigUint::from(1u64) << DROP_SHIFT))
        + keep * T::from_bn(&(BigUint::from(1u64) << KEEP_SHIFT))
        + dst_pc
}

pub fn encode_elem_entry<T: FromBn>(table_idx: T, type_idx: T, offset: T, func_idx: T) -> T {
    const TABLE_INDEX_SHIFT: u32 = TYPE_INDEX_SHIFT + COMMON_RANGE_BITS;
    const TYPE_INDEX_SHIFT: u32 = OFFSET_SHIFT + COMMON_RANGE_BITS;
    const OFFSET_SHIFT: u32 = FUNC_INDEX + FID_BITS;
    const FUNC_INDEX: u32 = 0;

    assert!(TABLE_INDEX_SHIFT + COMMON_RANGE_BITS <= INDIRECT_CLASS_SHIFT);

    T::from_bn(&(BigUint::from(IndirectClass::CallIndirect as u64)))
        * T::from_bn(&INDIRECT_CLASS_SHIFT_BN)
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
