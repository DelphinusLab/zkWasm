use num_bigint::ToBigUint;

use crate::encode::br_table::BR_TABLE_ENCODE_BOUNDARY;
use crate::encode::init_memory_table::INIT_MEMORY_ENCODE_BOUNDARY;
use crate::encode::instruction_table::INSTRUCTION_ENCODE_BOUNDARY;

use super::FromBn;

#[derive(Clone, Copy)]
pub enum ImageTableEncoder {
    Instruction = 1,
    BrTable = 2,
    InitMemory = 3,
}

impl ImageTableEncoder {
    pub fn encode<T: FromBn>(&self, data: T) -> T {
        const CLASS_SHIFT: u32 = 224;

        assert!(INSTRUCTION_ENCODE_BOUNDARY <= CLASS_SHIFT);
        assert!(BR_TABLE_ENCODE_BOUNDARY <= CLASS_SHIFT);
        assert!(INIT_MEMORY_ENCODE_BOUNDARY <= CLASS_SHIFT);

        T::from_bn(&(*self as u64).to_biguint().unwrap())
            * T::from_bn(&(1u64.to_biguint().unwrap() << CLASS_SHIFT))
            + data
    }
}
