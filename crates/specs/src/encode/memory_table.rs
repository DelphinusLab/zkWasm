use num_bigint::BigUint;
use num_traits::One;
use static_assertions::const_assert;

use super::FromBn;
use crate::encode::U32_BITS;

const _END_SHIFT: u32 = OFFSET_SHIFT + U32_BITS;
const OFFSET_SHIFT: u32 = LOCATION_TYPE_SHIFT + U32_BITS;
const LOCATION_TYPE_SHIFT: u32 = IS_I32_SHIFT + 1;
const IS_I32_SHIFT: u32 = 0;

const_assert!(_END_SHIFT < 240);

lazy_static! {
    pub static ref MEMORY_TABLE_ENTRY_OFFSET: BigUint = BigUint::one() << OFFSET_SHIFT;
    pub static ref MEMORY_TABLE_ENTRY_LOCATION_TYPE: BigUint =
        BigUint::one() << LOCATION_TYPE_SHIFT;
    pub static ref MEMORY_TABLE_ENTRY_IS_I32: BigUint = BigUint::one() << IS_I32_SHIFT;
}

pub fn encode_memory_table_entry<T: FromBn>(offset: T, location_type: T, is_i32: T) -> T {
    offset * T::from_bn(&MEMORY_TABLE_ENTRY_OFFSET)
        + location_type * T::from_bn(&MEMORY_TABLE_ENTRY_LOCATION_TYPE)
        + is_i32 * T::from_bn(&MEMORY_TABLE_ENTRY_IS_I32)
}
