use num_bigint::BigUint;

use crate::encode::FromBn;

pub fn encode_host_call_entry<T: FromBn>(host_call_idx: T, op: T, is_ret: T, arg: T) -> T {
    host_call_idx * T::from_bn(&(BigUint::from(1u64) << 96))
        + op * T::from_bn(&(BigUint::from(1u64) << 80))
        + is_ret * T::from_bn(&(BigUint::from(1u64) << 64))
        + arg
}
