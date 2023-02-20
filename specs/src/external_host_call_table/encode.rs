use num_bigint::BigUint;

use crate::{encode::FromBn, utils::common_range::COMMON_RANGE_OFFSET};

pub fn encode_host_call_entry<T: FromBn>(host_call_idx: T, op: T, is_ret: T, arg: T) -> T {
    const ARG_OFFSET: u32 = 0;
    const IS_RET_OFFSET: u32 = ARG_OFFSET + 64;
    const OP_OFFSET: u32 = IS_RET_OFFSET + COMMON_RANGE_OFFSET;
    const HOST_CALL_INDEX_OFFSET: u32 = OP_OFFSET + COMMON_RANGE_OFFSET;

    host_call_idx * T::from_bn(&(BigUint::from(1u64) << HOST_CALL_INDEX_OFFSET))
        + op * T::from_bn(&(BigUint::from(1u64) << OP_OFFSET))
        + is_ret * T::from_bn(&(BigUint::from(1u64) << IS_RET_OFFSET))
        + arg
}
