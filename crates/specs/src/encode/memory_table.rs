use num_bigint::ToBigUint;

use super::FromBn;
use crate::encode::COMMON_RANGE_OFFSET;

pub fn encode_memory_table_entry<T: FromBn>(offset: T, location_type: T, is_i32: T) -> T {
    const END_SHIFT: u32 = OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const OFFSET_SHIFT: u32 = LOCATION_TYPE_SHIFT + COMMON_RANGE_OFFSET;
    const LOCATION_TYPE_SHIFT: u32 = IS_I32_SHIFT + 1;
    const IS_I32_SHIFT: u32 = 0;

    assert!(END_SHIFT < 240);

    offset * T::from_bn(&(1u64.to_biguint().unwrap() << OFFSET_SHIFT))
        + location_type * T::from_bn(&(1u64.to_biguint().unwrap() << LOCATION_TYPE_SHIFT))
        + is_i32 * T::from_bn(&(1u64.to_biguint().unwrap() << IS_I32_SHIFT))
}
