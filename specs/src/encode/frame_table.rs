use num_bigint::ToBigUint;

use crate::utils::common_range::COMMON_RANGE_OFFSET;

use super::FromBn;

pub fn encode_frame_table_entry<T: FromBn>(
    frame_id: T,
    last_frame_id: T,
    callee_fid: T,
    fid: T,
    iid: T,
) -> T {
    const IID_SHIFT: u32 = 0;
    const FID_SHIFT: u32 = IID_SHIFT + COMMON_RANGE_OFFSET;
    const CALLEE_FID: u32 = FID_SHIFT + COMMON_RANGE_OFFSET;
    const LAST_JUMP_EID_SHIFT: u32 = CALLEE_FID + COMMON_RANGE_OFFSET;
    const EID_SHIFT: u32 = LAST_JUMP_EID_SHIFT + COMMON_RANGE_OFFSET;

    frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << EID_SHIFT))
        + last_frame_id * T::from_bn(&(1u64.to_biguint().unwrap() << LAST_JUMP_EID_SHIFT))
        + callee_fid * T::from_bn(&(1u64.to_biguint().unwrap() << CALLEE_FID))
        + fid * T::from_bn(&(1u64.to_biguint().unwrap() << FID_SHIFT))
        + iid
}
