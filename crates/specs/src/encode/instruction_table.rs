use num_bigint::BigUint;
use num_bigint::ToBigUint;
use static_assertions::const_assert;

use crate::itable::InstructionTableEntry;
use crate::itable::Opcode;
use crate::itable::OPCODE_SHIFT;

use super::FromBn;

pub const INSTRUCTION_ENCODE_BOUNDARY: u32 = 254;
pub(crate) const IID_BITS: u32 = 16;
pub(crate) const FID_BITS: u32 = 16;
const_assert!(OPCODE_SHIFT + IID_BITS + FID_BITS <= INSTRUCTION_ENCODE_BOUNDARY);

pub fn encode_instruction_table_entry<T: FromBn>(fid: T, iid: T, opcode: T) -> T {
    const FID_SHIFT: u32 = IID_SHIFT + IID_BITS;
    const IID_SHIFT: u32 = OPCODE_SHIFT;

    fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_SHIFT))
        + iid * T::from_bn(&(1u64.to_biguint().unwrap() << IID_SHIFT))
        + opcode
}

impl InstructionTableEntry {
    pub(crate) fn encode(fid: u32, iid: u32, opcode: &Opcode) -> BigUint {
        assert!(fid <= 1 << FID_BITS);
        assert!(iid <= 1 << IID_BITS);

        encode_instruction_table_entry(BigUint::from(fid), BigUint::from(iid), opcode.into())
    }
}
