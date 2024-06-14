use halo2_proofs::arithmetic::FieldExt;
use num_bigint::BigUint;
use num_bigint::ToBigUint;
use num_traits::identities::One;
use static_assertions::const_assert;
use static_assertions::const_assert_eq;

use super::bn_to_field;
use super::FromBn;
use crate::imtable::InitMemoryTableEntry;

pub const INIT_MEMORY_ENCODE_BOUNDARY: u32 = 224;
pub const MEMORY_ADDRESS_OFFSET: u32 = 97;

const LTYPE_SHIFT: u32 = OFFSET_SHIFT + u32::BITS;
const OFFSET_SHIFT: u32 = IS_MUTABLE_SHIFT + 1;
const IS_MUTABLE_SHIFT: u32 = EID_OFFSET_SHIFT + u32::BITS;
const EID_OFFSET_SHIFT: u32 = VALUE_SHIFT + u64::BITS;
const VALUE_SHIFT: u32 = 0;

const_assert_eq!(OFFSET_SHIFT, MEMORY_ADDRESS_OFFSET);
const_assert!(LTYPE_SHIFT + 8 <= INIT_MEMORY_ENCODE_BOUNDARY);

lazy_static! {
    static ref IS_MUTABLE_OFFSET: BigUint = BigUint::one() << IS_MUTABLE_SHIFT;
    static ref EID_OFFSET: BigUint = BigUint::one() << EID_OFFSET_SHIFT;
    static ref LTYPE_OFFSET: BigUint = BigUint::one() << LTYPE_SHIFT;
    static ref MEMORY_OFFSET_OFFSET: BigUint = BigUint::one() << OFFSET_SHIFT;
    static ref MEMORY_LOCATION_OFFSET: BigUint = BigUint::one() << 32;
}

pub fn encode_init_memory_table_address<T: FromBn>(location_type: T, offset: T) -> T {
    location_type * T::from_bn(&MEMORY_LOCATION_OFFSET) + offset
}

pub fn encode_init_memory_table_entry<T: FromBn>(
    ltype: T,
    offset: T,
    is_mutable: T,
    eid: T,
    value: T,
) -> T {
    let encode =
        is_mutable * T::from_bn(&IS_MUTABLE_OFFSET) + eid * T::from_bn(&EID_OFFSET) + value;

    if cfg!(feature = "continuation") {
        encode
    } else {
        ltype * T::from_bn(&LTYPE_OFFSET) + offset * T::from_bn(&MEMORY_OFFSET_OFFSET) + encode
    }
}

pub fn init_memory_table_entry_encode_update_offset<T: FieldExt>(
    encode: T,
    offset: impl FnOnce() -> T,
) -> T {
    if cfg!(feature = "continuation") {
        encode
    } else {
        encode + offset() * bn_to_field::<T>(&MEMORY_OFFSET_OFFSET)
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
