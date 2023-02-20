use num_bigint::{BigUint, ToBigUint};

use crate::{
    itable::{InstructionTableEntry, OPCODE_BITS},
    utils::common_range::COMMON_RANGE_OFFSET,
};

use super::FromBn;

pub fn encode_instruction_table_entry<T: FromBn>(fid: T, iid: T, opcode: T) -> T {
    const OPCODE_OFFSET: u32 = 0;
    const IID_OFFSET: u32 = OPCODE_OFFSET + OPCODE_BITS;
    const FID_OFFSET: u32 = IID_OFFSET + COMMON_RANGE_OFFSET;

    fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_OFFSET))
        + iid * T::from_bn(&(1u64.to_biguint().unwrap() << IID_OFFSET))
        + opcode
}

impl InstructionTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_instruction_table_entry(
            BigUint::from(self.fid),
            BigUint::from(self.iid),
            self.opcode.clone().into(),
        )
    }
}
