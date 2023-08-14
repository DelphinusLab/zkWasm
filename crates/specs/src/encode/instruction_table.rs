use num_bigint::BigUint;
use num_bigint::ToBigUint;

use crate::encode::COMMON_RANGE_OFFSET;
use crate::itable::InstructionTableEntry;
use crate::itable::OPCODE_SHIFT;

use super::FromBn;

pub(crate) const INSTRUCTION_ENCODE_BOUNDARY: u32 = 224;

pub fn encode_instruction_table_entry<T: FromBn>(fid: T, iid: T, opcode: T) -> T {
    const FID_SHIFT: u32 = IID_SHIFT + COMMON_RANGE_OFFSET;
    const IID_SHIFT: u32 = OPCODE_SHIFT;

    assert!(FID_SHIFT + COMMON_RANGE_OFFSET <= INSTRUCTION_ENCODE_BOUNDARY);

    fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_SHIFT))
        + iid * T::from_bn(&(1u64.to_biguint().unwrap() << IID_SHIFT))
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
