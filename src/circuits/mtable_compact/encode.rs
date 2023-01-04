use std::ops::{Add, Mul};

use halo2_proofs::{arithmetic::FieldExt, plonk::Expression};
use num_bigint::BigUint;
use specs::mtable::{AccessType, LocationType};

use crate::{
    circuits::{utils::bn_to_field, FromBn},
    constant_from, constant_from_bn,
};

lazy_static! {
    static ref VAR_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 64;
    static ref ACCESS_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 77;
    static ref LOC_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 79;
    static ref OFFSET_SHIFT: BigUint = BigUint::from(1u64) << 80;
    static ref MMID_SHIFT: BigUint = BigUint::from(1u64) << 96;
    static ref EMID_SHIFT: BigUint = BigUint::from(1u64) << 112;
    static ref EID_SHIFT: BigUint = BigUint::from(1u64) << 128;
}
pub(crate) struct MemoryTableLookupEncode {}

impl MemoryTableLookupEncode {
    pub(super) fn encode_for_lookup<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        mmid: T,
        offset: T,
        ltype: T,
        atype: T,
        vtype: T,
        value: T,
    ) -> T {
        eid * T::from_bn(&EID_SHIFT)
            + emid * T::from_bn(&EMID_SHIFT)
            + mmid * T::from_bn(&MMID_SHIFT)
            + offset * T::from_bn(&OFFSET_SHIFT)
            + ltype * T::from_bn(&LOC_TYPE_SHIFT)
            + atype * T::from_bn(&ACCESS_TYPE_SHIFT)
            + vtype * T::from_bn(&VAR_TYPE_SHIFT)
            + value
    }
}

impl<F: FieldExt> FromBn for Expression<F> {
    fn from_bn(bn: &BigUint) -> Self {
        constant_from_bn!(bn)
    }

    fn zero() -> Self {
        constant_from!(0)
    }
}

impl FromBn for BigUint {
    fn from_bn(bn: &BigUint) -> Self {
        bn.clone()
    }

    fn zero() -> Self {
        BigUint::from(0u64)
    }
}

impl MemoryTableLookupEncode {
    pub(crate) fn encode_stack_read<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        sp: T,
        vtype: T,
        value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            T::zero(),
            sp,
            T::from_bn(&BigUint::from(LocationType::Stack as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_stack_write<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        sp: T,
        vtype: T,
        value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            T::zero(),
            sp,
            T::from_bn(&BigUint::from(LocationType::Stack as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_memory_load<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        mmid: T,
        address: T,
        vtype: T,
        block_value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            mmid,
            address,
            T::from_bn(&BigUint::from(LocationType::Heap as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            block_value,
        )
    }

    pub(crate) fn encode_memory_store<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        mmid: T,
        address: T,
        vtype: T,
        block_value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            mmid,
            address,
            T::from_bn(&BigUint::from(LocationType::Heap as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            block_value,
        )
    }

    pub(crate) fn encode_global_get<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        mmid: T,
        address: T,
        vtype: T,
        value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            mmid,
            address,
            T::from_bn(&BigUint::from(LocationType::Global as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_global_set<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        eid: T,
        emid: T,
        mmid: T,
        address: T,
        vtype: T,
        value: T,
    ) -> T {
        MemoryTableLookupEncode::encode_for_lookup(
            eid,
            emid,
            mmid,
            address,
            T::from_bn(&BigUint::from(LocationType::Global as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            value,
        )
    }
}
