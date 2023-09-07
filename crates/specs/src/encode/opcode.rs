use num_bigint::BigUint;

use crate::itable::OpcodeClass;
use crate::itable::OPCODE_ARG0_SHIFT as OPCODE_ARG0;
use crate::itable::OPCODE_ARG1_SHIFT as OPCODE_ARG1;
use crate::itable::OPCODE_CLASS_SHIFT as OPCODE_CLASS;

use super::FromBn;

lazy_static! {
    static ref OPCODE_CLASS_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_CLASS;
    static ref OPCODE_ARG0_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_ARG0;
    static ref OPCODE_ARG1_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_ARG1;
}

pub fn encode_global_get<T: FromBn>(globalidx: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::GlobalGet as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + globalidx
}

pub fn encode_global_set<T: FromBn>(globalidx: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::GlobalSet as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + globalidx
}

pub fn encode_call<T: FromBn>(function_index: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::Call as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + function_index * T::from_bn(&OPCODE_ARG0_SHIFT)
}

pub fn encode_call_indirect<T: FromBn>(type_index: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::CallIndirect as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + type_index * T::from_bn(&OPCODE_ARG0_SHIFT)
}

pub fn encode_call_host<T: FromBn>(op: T, is_ret: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::CallHost as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + op * T::from_bn(&OPCODE_ARG0_SHIFT)
        + is_ret * T::from_bn(&OPCODE_ARG1_SHIFT)
}

pub fn encode_br<T: FromBn>(drop: T, keep: T, dst_pc: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::Br as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + drop * T::from_bn(&OPCODE_ARG0_SHIFT)
        + keep * T::from_bn(&OPCODE_ARG1_SHIFT)
        + dst_pc
}

pub fn encode_br_if_eqz<T: FromBn>(drop: T, keep: T, dst_pc: T) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::BrIfEqz as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + drop * T::from_bn(&OPCODE_ARG0_SHIFT)
        + keep * T::from_bn(&OPCODE_ARG1_SHIFT)
        + dst_pc
}

pub fn encode_br_table<T: FromBn>(len: T) -> T {
    T::from_bn(&BigUint::from(OpcodeClass::BrTable as u64)) + len
}

pub fn encode_conversion<T: FromBn>(
    sign: T,
    value_type_is_i32: T,
    value_is_i8: T,
    value_is_i16: T,
    value_is_i32: T,
    value_is_i64: T,
    res_is_i32: T,
    res_is_i64: T,
) -> T {
    T::from_bn(&(BigUint::from(OpcodeClass::Conversion as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT)
        + sign * T::from_bn(&BigUint::from(1u64 << 7))
        + value_type_is_i32 * T::from_bn(&BigUint::from(1u64 << 6))
        + value_is_i8 * T::from_bn(&BigUint::from(1u64 << 5))
        + value_is_i16 * T::from_bn(&BigUint::from(1u64 << 4))
        + value_is_i32 * T::from_bn(&BigUint::from(1u64 << 3))
        + value_is_i64 * T::from_bn(&BigUint::from(1u64 << 2))
        + res_is_i32 * T::from_bn(&BigUint::from(1u64 << 1))
        + res_is_i64
}
