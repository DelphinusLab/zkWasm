use num_bigint::{BigUint, ToBigUint};

use crate::{imtable::InitMemoryTableEntry, utils::common_range::COMMON_RANGE_OFFSET};

use super::FromBn;

pub fn encode_init_memory_table_entry<T: FromBn>(
    ltype: T,
    is_mutable: T,
    offset: T,
    value: T,
) -> T {
    const VALUE_SHIFT: u32 = 0;
    const OFFSET_SHIFT: u32 = VALUE_SHIFT + 64;
    const IS_MUTABLE_SHIFT: u32 = OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const LTYPE_SHIFT: u32 = IS_MUTABLE_SHIFT + COMMON_RANGE_OFFSET;

    ltype * T::from_bn(&(1u64.to_biguint().unwrap() << LTYPE_SHIFT))
        + is_mutable * T::from_bn(&(1u64.to_biguint().unwrap() << IS_MUTABLE_SHIFT))
        + offset * T::from_bn(&(1u64.to_biguint().unwrap() << OFFSET_SHIFT))
        + value
}

impl InitMemoryTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_init_memory_table_entry(
            BigUint::from(self.ltype as u32),
            BigUint::from(self.is_mutable as u32),
            BigUint::from(self.offset),
            self.value.to_biguint().unwrap(),
        )
    }
}
