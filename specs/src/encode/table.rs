use super::FromBn;
use num_bigint::BigUint;

pub fn encode_br_table_entry<T: FromBn>(
    moid: T,
    fid: T,
    iid: T,
    index: T,
    drop: T,
    keep: T,
    dst_pc: T,
) -> T {
    moid * T::from_bn(&(BigUint::from(1u64) << 112))
        + fid * T::from_bn(&(BigUint::from(1u64) << 96))
        + iid * T::from_bn(&(BigUint::from(1u64) << 80))
        + index * T::from_bn(&(BigUint::from(1u64) << 64))
        + drop * T::from_bn(&(BigUint::from(1u64) << 48))
        + keep * T::from_bn(&(BigUint::from(1u64) << 32))
        + dst_pc
}
