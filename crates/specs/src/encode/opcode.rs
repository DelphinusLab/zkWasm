use num_bigint::BigUint;
use num_traits::One;

use crate::itable::Opcode;
use crate::itable::OpcodeClass;
use crate::itable::OpcodeClassPlain;
use crate::itable::UniArg;
use crate::itable::OPCODE_CLASS_SHIFT as OPCODE_CLASS;

use super::instruction_table::FID_BITS;
use super::FromBn;

const UNIARG_BITS: u32 = 66;

lazy_static! {
    static ref OPCODE_CLASS_SHIFT: BigUint = BigUint::from(1u64) << OPCODE_CLASS;
    static ref OPCODE_U64_SHIFT: BigUint = BigUint::one() << u64::BITS;
    static ref OPCODE_U32_SHIFT: BigUint = BigUint::one() << u32::BITS;
    static ref OPCODE_UNIARG_SHIFT: BigUint = BigUint::from(1u64) << UNIARG_BITS;
    static ref OPCODE_FUNC_SHIFT: BigUint = BigUint::one() << FID_BITS;
    static ref OPCODE_BIT_SHIFT: BigUint = BigUint::one() << 1;
}

enum EncoderType {
    U64,
    U32,
    Func,
    Bit,
}

impl EncoderType {
    fn bits(&self) -> u32 {
        match self {
            EncoderType::U64 => u64::BITS,
            EncoderType::U32 => u32::BITS,
            EncoderType::Func => FID_BITS,
            EncoderType::Bit => 1,
        }
    }

    fn shift(&self) -> &BigUint {
        match self {
            EncoderType::U64 => &*OPCODE_U64_SHIFT,
            EncoderType::U32 => &*OPCODE_U32_SHIFT,
            EncoderType::Func => &*OPCODE_FUNC_SHIFT,
            EncoderType::Bit => &*OPCODE_BIT_SHIFT,
        }
    }
}

pub enum UniArgEncode<const N: usize, T: FromBn> {
    Reserve,
    Value([T; N]),
}

impl From<&UniArg> for UniArgEncode<1, BigUint> {
    fn from(uniarg: &UniArg) -> Self {
        UniArgEncode::Value([uniarg.encode()])
    }
}

impl<const N: usize> From<&[UniArg; N]> for UniArgEncode<N, BigUint> {
    fn from(uniargs: &[UniArg; N]) -> Self {
        UniArgEncode::Value(
            uniargs
                .iter()
                .map(|uniarg| uniarg.encode())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        )
    }
}

struct Encoder;

impl Encoder {
    fn encode_without_uniarg<T: FromBn>(
        opcode_class: OpcodeClass,
        values: Vec<(T, EncoderType)>,
    ) -> T {
        let mut encode = T::zero();
        let mut bits = 0;

        for (value, encoder_type) in values {
            encode = encode * T::from_bn(&encoder_type.shift()) + value;
            bits += encoder_type.bits();
        }

        assert!(bits <= OPCODE_CLASS);

        T::from_bn(&(BigUint::from(opcode_class as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT) + encode
    }

    fn encode<const N: usize, T: FromBn>(
        opcode_class: OpcodeClass,
        values: Vec<(T, EncoderType)>,
        uniargs: UniArgEncode<N, T>,
    ) -> T {
        let mut encode = T::zero();
        let mut bits = 0;

        for (value, encoder_type) in values {
            encode = encode * T::from_bn(&encoder_type.shift()) + value;
            bits += encoder_type.bits();
        }

        match uniargs {
            UniArgEncode::Reserve => {
                for _ in 0..N {
                    encode = encode * T::from_bn(&OPCODE_UNIARG_SHIFT);
                    bits += UNIARG_BITS;
                }
            }
            UniArgEncode::Value(uniargs) => {
                for uniarg in uniargs {
                    encode = encode * T::from_bn(&OPCODE_UNIARG_SHIFT) + uniarg;
                    bits += UNIARG_BITS;
                }
            }
        }

        assert!(bits <= OPCODE_CLASS);

        T::from_bn(&(BigUint::from(opcode_class as u64))) * T::from_bn(&OPCODE_CLASS_SHIFT) + encode
    }
}

pub fn encode_local_get<T: FromBn>(vtype: T, offset: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::LocalGet,
        vec![(vtype, EncoderType::Bit), (offset, EncoderType::U32)],
    )
}

pub fn encode_local_set<T: FromBn>(vtype: T, offset: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::LocalSet,
        vec![(vtype, EncoderType::Bit), (offset, EncoderType::U32)],
        uniarg,
    )
}

pub fn encode_local_tee<T: FromBn>(vtype: T, offset: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::LocalTee,
        vec![(vtype, EncoderType::Bit), (offset, EncoderType::U32)],
    )
}

pub fn encode_global_get<T: FromBn>(globalidx: T) -> T {
    Encoder::encode_without_uniarg(OpcodeClass::GlobalGet, vec![(globalidx, EncoderType::U32)])
}

pub fn encode_global_set<T: FromBn>(globalidx: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::GlobalSet,
        vec![(globalidx, EncoderType::U32)],
        uniarg,
    )
}

pub fn encode_const<T: FromBn>(vtype: T, value: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::Const,
        vec![(vtype, EncoderType::Bit), (value, EncoderType::U64)],
    )
}

pub fn encode_drop<T: FromBn>() -> T {
    Encoder::encode_without_uniarg(OpcodeClass::Drop, vec![])
}

pub fn encode_select<T: FromBn>(uniargs: UniArgEncode<3, T>) -> T {
    Encoder::encode(OpcodeClass::Select, vec![], uniargs)
}

pub fn encode_return<T: FromBn>(drop: T, keep: T, vtype: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::Return,
        vec![
            (drop, EncoderType::U32),
            (keep, EncoderType::U32),
            (vtype, EncoderType::Bit),
        ],
    )
}

pub fn encode_bin<T: FromBn>(class: T, vtype: T, uniargs: UniArgEncode<2, T>) -> T {
    Encoder::encode(
        OpcodeClass::Bin,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniargs,
    )
}

pub fn encode_bin_shift<T: FromBn>(class: T, vtype: T, uniargs: UniArgEncode<2, T>) -> T {
    Encoder::encode(
        OpcodeClass::BinShift,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniargs,
    )
}

pub fn encode_bin_bit<T: FromBn>(class: T, vtype: T, uniargs: UniArgEncode<2, T>) -> T {
    Encoder::encode(
        OpcodeClass::BinBit,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniargs,
    )
}

pub fn encode_unary<T: FromBn>(class: T, vtype: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::Unary,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniarg,
    )
}

pub fn encode_test<T: FromBn>(class: T, vtype: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::Test,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniarg,
    )
}

pub fn encode_rel<T: FromBn>(class: T, vtype: T, uniargs: UniArgEncode<2, T>) -> T {
    Encoder::encode(
        OpcodeClass::Rel,
        vec![(class, EncoderType::U32), (vtype, EncoderType::Bit)],
        uniargs,
    )
}

pub fn encode_br<T: FromBn>(drop: T, keep: T, dst_pc: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::Br,
        vec![
            (drop, EncoderType::U32),
            (keep, EncoderType::U32),
            (dst_pc, EncoderType::U32),
        ],
    )
}

pub fn encode_br_if<T: FromBn>(drop: T, keep: T, dst_pc: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::BrIf,
        vec![
            (drop, EncoderType::U32),
            (keep, EncoderType::U32),
            (dst_pc, EncoderType::U32),
        ],
        uniarg,
    )
}

pub fn encode_br_if_eqz<T: FromBn>(drop: T, keep: T, dst_pc: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::BrIfEqz,
        vec![
            (drop, EncoderType::U32),
            (keep, EncoderType::U32),
            (dst_pc, EncoderType::U32),
        ],
        uniarg,
    )
}

pub fn encode_br_table<T: FromBn>(len: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(OpcodeClass::BrTable, vec![(len, EncoderType::U32)], uniarg)
}

pub fn encode_unreachable<T: FromBn>() -> T {
    Encoder::encode_without_uniarg(OpcodeClass::Unreachable, vec![])
}

pub fn encode_call<T: FromBn>(function_index: T) -> T {
    Encoder::encode_without_uniarg(OpcodeClass::Call, vec![(function_index, EncoderType::Func)])
}

pub fn encode_call_indirect<T: FromBn>(type_index: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::CallIndirect,
        vec![(type_index, EncoderType::U32)],
        uniarg,
    )
}

pub fn encode_call_host<T: FromBn>(op: T, is_ret: T) -> T {
    Encoder::encode_without_uniarg(
        OpcodeClass::CallHost,
        vec![(op, EncoderType::U32), (is_ret, EncoderType::Bit)],
    )
}

pub fn encode_call_internal_host<T: FromBn>(opcode: &Opcode, op_index_in_plugin: usize) -> T {
    let opcode_class_plain: OpcodeClassPlain = opcode.into();

    T::from_bn(&(BigUint::from(opcode_class_plain.0) << OPCODE_CLASS))
        + T::from_bn(&BigUint::from(op_index_in_plugin as u64))
}

pub fn encode_load<T: FromBn>(vtype: T, size: T, offset: T, uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(
        OpcodeClass::Load,
        vec![
            (vtype, EncoderType::U32),
            (size, EncoderType::U32),
            (offset, EncoderType::U32),
        ],
        uniarg,
    )
}

pub fn encode_store<T: FromBn>(vtype: T, size: T, offset: T, uniargs: UniArgEncode<2, T>) -> T {
    Encoder::encode(
        OpcodeClass::Store,
        vec![
            (vtype, EncoderType::Bit),
            (size, EncoderType::U32),
            (offset, EncoderType::U32),
        ],
        uniargs,
    )
}

pub fn encode_memory_size<T: FromBn>() -> T {
    Encoder::encode_without_uniarg(OpcodeClass::MemorySize, vec![])
}

pub fn encode_memory_grow<T: FromBn>(uniarg: UniArgEncode<1, T>) -> T {
    Encoder::encode(OpcodeClass::MemoryGrow, vec![], uniarg)
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
    uniarg: UniArgEncode<1, T>,
) -> T {
    Encoder::encode(
        OpcodeClass::Conversion,
        vec![
            (sign, EncoderType::Bit),
            (value_type_is_i32, EncoderType::Bit),
            (value_is_i8, EncoderType::Bit),
            (value_is_i16, EncoderType::Bit),
            (value_is_i32, EncoderType::Bit),
            (value_is_i64, EncoderType::Bit),
            (res_is_i32, EncoderType::Bit),
            (res_is_i64, EncoderType::Bit),
        ],
        uniarg,
    )
}
