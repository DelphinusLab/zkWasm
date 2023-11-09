use num_bigint::BigUint;
use num_bigint::ToBigUint;

use super::FromBn;
use crate::encode::COMMON_RANGE_OFFSET;
use crate::imtable::InitMemoryTableEntry;

pub(crate) const INIT_MEMORY_ENCODE_BOUNDARY: u32 = 224;

pub fn encode_init_memory_table_entry<T: FromBn>(
    ltype: T,
    is_mutable: T,
    start_offset: T,
    end_offset: T,
    eid: T,
    value: T,
) -> T {
    const LTYPE_SHIFT: u32 = IS_MUTABLE_SHIFT + 8;
    const IS_MUTABLE_SHIFT: u32 = START_OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const START_OFFSET_SHIFT: u32 = END_OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const END_OFFSET_SHIFT: u32 = EID_OFFSET_SHIFT + 32;
    const EID_OFFSET_SHIFT: u32 = VALUE_SHIFT + 64;
    const VALUE_SHIFT: u32 = 0;

    assert!(LTYPE_SHIFT + 8 <= INIT_MEMORY_ENCODE_BOUNDARY);

    ltype * T::from_bn(&(1u64.to_biguint().unwrap() << LTYPE_SHIFT))
        + is_mutable * T::from_bn(&(1u64.to_biguint().unwrap() << IS_MUTABLE_SHIFT))
        + start_offset * T::from_bn(&(1u64.to_biguint().unwrap() << START_OFFSET_SHIFT))
        + end_offset * T::from_bn(&(1u64.to_biguint().unwrap() << END_OFFSET_SHIFT))
        + eid * T::from_bn(&(1u64.to_biguint().unwrap() << EID_OFFSET_SHIFT))
        + value
}

impl InitMemoryTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_init_memory_table_entry(
            BigUint::from(self.ltype as u32),
            BigUint::from(self.is_mutable as u32),
            BigUint::from(self.start_offset),
            BigUint::from(self.end_offset),
            BigUint::from(self.eid),
            self.value.to_biguint().unwrap(),
        )
    }
}
