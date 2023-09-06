use num_bigint::BigUint;

use crate::itable::ConversionOp;
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

pub fn encode_conversion<T: FromBn>(op: ConversionOp) -> T {
    macro_rules! encode {
        ($e:ident) => {
            T::from_bn(&(BigUint::from(OpcodeClass::Conversion as u64)))
                * T::from_bn(&OPCODE_CLASS_SHIFT)
                + T::from_bn(&(BigUint::from(ConversionOp::$e as u64)))
                    * T::from_bn(&OPCODE_ARG0_SHIFT)
        };
    }

    match op {
        ConversionOp::I32WrapI64 => encode!(I32WrapI64),
        ConversionOp::I64ExtendI32s => encode!(I64ExtendI32s),
        ConversionOp::I64ExtendI32u => encode!(I64ExtendI32u),
        ConversionOp::I32Extend8S => encode!(I32Extend8S),
        ConversionOp::I32Extend16S => encode!(I32Extend16S),
        ConversionOp::I64Extend8S => encode!(I64Extend8S),
        ConversionOp::I64Extend16S => encode!(I64Extend16S),
        ConversionOp::I64Extend32S => encode!(I64Extend32S),
    }
}
