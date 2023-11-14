use num_bigint::BigUint;
use num_bigint::ToBigUint;

use super::FromBn;
use crate::imtable::InitMemoryTableEntry;

pub(crate) const INIT_MEMORY_ENCODE_BOUNDARY: u32 = 224;

pub fn encode_init_memory_table_address<T: FromBn>(location_type: T, offset: T) -> T {
    location_type * T::from_bn(&(1u64.to_biguint().unwrap() << 32)) + offset
}

pub fn encode_init_memory_table_entry<T: FromBn>(
    ltype: T,
    offset: T,
    is_mutable: T,
    eid: T,
    value: T,
) -> T {
    const LTYPE_SHIFT: u32 = OFFSET_SHIFT + 32;
    const OFFSET_SHIFT: u32 = IS_MUTABLE_SHIFT + 1;
    const IS_MUTABLE_SHIFT: u32 = EID_OFFSET_SHIFT + 32;
    const EID_OFFSET_SHIFT: u32 = VALUE_SHIFT + 64;
    const VALUE_SHIFT: u32 = 0;

    assert!(LTYPE_SHIFT + 8 <= INIT_MEMORY_ENCODE_BOUNDARY);

    if cfg!(feature = "continuation") {
        ltype * T::from_bn(&(1u64.to_biguint().unwrap() << LTYPE_SHIFT))
            + offset * T::from_bn(&(1u64.to_biguint().unwrap() << OFFSET_SHIFT))
            + is_mutable * T::from_bn(&(1u64.to_biguint().unwrap() << IS_MUTABLE_SHIFT))
            + eid * T::from_bn(&(1u64.to_biguint().unwrap() << EID_OFFSET_SHIFT))
            + value
    } else {
        is_mutable * T::from_bn(&(1u64.to_biguint().unwrap() << IS_MUTABLE_SHIFT))
            + eid * T::from_bn(&(1u64.to_biguint().unwrap() << EID_OFFSET_SHIFT))
            + value
    }
}

impl InitMemoryTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_init_memory_table_entry(
            BigUint::from(self.ltype as u32),
            BigUint::from(self.offset),
            BigUint::from(self.is_mutable as u32),
            BigUint::from(self.eid),
            self.value.to_biguint().unwrap(),
        )
    }
}
