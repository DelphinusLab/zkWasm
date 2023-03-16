use num_bigint::BigUint;
use num_bigint::ToBigUint;

use crate::encode::COMMON_RANGE_OFFSET;
use crate::jtable::JumpTableEntry;
use crate::jtable::StaticFrameEntry;

use super::FromBn;

pub fn encode_frame_table_entry<T: FromBn>(
    frame_id: T,
    last_frame_id: T,
    callee_fid: T,
    fid: T,
    iid: T,
) -> T {
    const FRAME_ID_SHIFT: u32 = LAST_JUMP_FRAME_ID_SHIFT + COMMON_RANGE_OFFSET;
    const LAST_JUMP_FRAME_ID_SHIFT: u32 = CALLEE_FID + COMMON_RANGE_OFFSET;
    const CALLEE_FID: u32 = FID_SHIFT + COMMON_RANGE_OFFSET;
    const FID_SHIFT: u32 = IID_SHIFT + COMMON_RANGE_OFFSET;
    const IID_SHIFT: u32 = 0;

    frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << FRAME_ID_SHIFT))
        + last_frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << LAST_JUMP_FRAME_ID_SHIFT))
        + callee_fid * T::from_bn(&(1u64.to_biguint().unwrap() << CALLEE_FID))
        + fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_SHIFT))
        + iid
}

impl StaticFrameEntry {
    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.frame_id.to_biguint().unwrap(),
            self.next_frame_id.to_biguint().unwrap(),
            self.callee_fid.to_biguint().unwrap(),
            self.fid.to_biguint().unwrap(),
            self.iid.to_biguint().unwrap(),
        )
    }
}

impl JumpTableEntry {
    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.eid.to_biguint().unwrap(),
            self.last_jump_eid.to_biguint().unwrap(),
            self.callee_fid.to_biguint().unwrap(),
            self.inst.fid.to_biguint().unwrap(),
            self.inst.iid.to_biguint().unwrap(),
        )
    }
}
