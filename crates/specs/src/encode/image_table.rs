use num_bigint::BigUint;
use num_bigint::ToBigUint;
use static_assertions::const_assert;

use crate::encode::br_table::BR_TABLE_ENCODE_BOUNDARY;
use crate::encode::init_memory_table::INIT_MEMORY_ENCODE_BOUNDARY;
use crate::encode::instruction_table::INSTRUCTION_ENCODE_BOUNDARY;

use super::FromBn;

const CLASS_SHIFT: u32 = 254;

#[derive(Clone, Copy)]
pub enum ImageTableEncoder {
    Instruction = 1,
    BrTable = 2,
    InitMemory = 3,
}

const_assert!(INSTRUCTION_ENCODE_BOUNDARY <= CLASS_SHIFT);
const_assert!(BR_TABLE_ENCODE_BOUNDARY <= CLASS_SHIFT);
const_assert!(INIT_MEMORY_ENCODE_BOUNDARY <= CLASS_SHIFT);

lazy_static! {
    static ref INSTRUCTION_TAG: BigUint = (ImageTableEncoder::Instruction as u64)
        .to_biguint()
        .unwrap()
        << CLASS_SHIFT;
    static ref BR_TABLE_TAG: BigUint =
        (ImageTableEncoder::BrTable as u64).to_biguint().unwrap() << CLASS_SHIFT;
    static ref INIT_MEMORY_TAG: BigUint =
        (ImageTableEncoder::InitMemory as u64).to_biguint().unwrap() << CLASS_SHIFT;
}

impl ImageTableEncoder {
    #[inline(always)]
    pub fn encode<T: FromBn>(&self, data: T) -> T {
        match self {
            ImageTableEncoder::Instruction => T::from_bn(&INSTRUCTION_TAG) + data,
            ImageTableEncoder::BrTable => T::from_bn(&BR_TABLE_TAG) + data,
            ImageTableEncoder::InitMemory => T::from_bn(&INIT_MEMORY_TAG) + data,
        }
    }
}
