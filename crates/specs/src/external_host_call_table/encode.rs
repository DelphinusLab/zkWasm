use num_bigint::BigUint;

use crate::encode::FromBn;

pub fn encode_host_call_entry<T: FromBn>(host_call_idx: T, opcode: T, operand: T) -> T {
    host_call_idx * T::from_bn(&(BigUint::from(1u64) << 96))
        + opcode * T::from_bn(&(BigUint::from(1u64) << 64))
        + operand
}
