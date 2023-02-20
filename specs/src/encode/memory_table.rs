use num_bigint::ToBigUint;

use super::FromBn;
use crate::utils::common_range::COMMON_RANGE_OFFSET;

pub fn encode_memory_table_entry<T: FromBn>(
    eid: T,
    emid: T,
    offset: T,
    location_type: T,
    access_type: T,
    var_type: T,
    value: T,
) -> T {
    const VALUE_SHIFT: u32 = 0;
    const VAR_TYPE_SHIFT: u32 = VALUE_SHIFT + 64;
    const ACCESS_TYPE_SHIFT: u32 = VAR_TYPE_SHIFT + COMMON_RANGE_OFFSET;
    const LOCATION_TYPE_SHIFT: u32 = ACCESS_TYPE_SHIFT + COMMON_RANGE_OFFSET;
    const OFFSET_SHIFT: u32 = LOCATION_TYPE_SHIFT + COMMON_RANGE_OFFSET;
    const EMID_SHIFT: u32 = OFFSET_SHIFT + COMMON_RANGE_OFFSET;
    const EID_SHIFT: u32 = EMID_SHIFT + COMMON_RANGE_OFFSET;

    eid * T::from_bn(&(1u64.to_biguint().unwrap() << EID_SHIFT))
        + emid * T::from_bn(&(1u64.to_biguint().unwrap() << EMID_SHIFT))
        + offset * T::from_bn(&(1u64.to_biguint().unwrap() << OFFSET_SHIFT))
        + location_type * T::from_bn(&(1u64.to_biguint().unwrap() << LOCATION_TYPE_SHIFT))
        + access_type * T::from_bn(&(1u64.to_biguint().unwrap() << ACCESS_TYPE_SHIFT))
        + var_type * T::from_bn(&(1u64.to_biguint().unwrap() << VAR_TYPE_SHIFT))
        + value
}
