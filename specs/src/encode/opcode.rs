use num_bigint::BigUint;

use crate::itable::{
    OpcodeClass, OPCODE_ARG0_SHIFT as OPCODE_ARG0, OPCODE_ARG1_SHIFT as OPCODE_ARG1,
    OPCODE_CLASS_SHIFT as OPCODE_CLASS,
};

use super::FromBn;

lazy_static! {
    static ref OPCODE_CLASS_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_CLASS;
    static ref OPCODE_ARG0_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_ARG0;
    static ref OPCODE_ARG1_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_ARG1;
}

pub fn encode_call<T: FromBn>(function_index: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::Call as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + function_index * T::from_bn(&OPCODE_ARG0_SHIFT)
}

pub fn encode_br_if_eqz<T: FromBn>(drop: T, keep: T, dst_pc: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::BrIfEqz as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + drop * T::from_bn(&OPCODE_ARG0_SHIFT)
        + keep * T::from_bn(&OPCODE_ARG1_SHIFT)
        + dst_pc
}
