use halo2_proofs::arithmetic::FieldExt;
use num_bigint::BigUint;
use specs::mtable::{AccessType, LocationType, VarType};
use std::ops::{Add, Mul};

use super::{
    MemoryTableConfig, ACCESS_TYPE_SHIFT, EID_SHIFT, EMID_SHIFT, LOC_TYPE_SHIFT, MMID_SHIFT,
    OFFSET_SHIFT, VAR_TYPE_SHIFT,
};

pub(crate) trait MtableLookupVTypeEncode {
    fn encode(&self) -> u16;
}

impl MtableLookupVTypeEncode for VarType {
    fn encode(&self) -> u16 {
        let ge_2_bytes = (self.byte_size() >= 2) as u16;
        let ge_4_bytes = (self.byte_size() >= 4) as u16;
        let ge_8_bytes = (self.byte_size() >= 8) as u16;
        let unsigned = (*self as u16) & 1;

        (ge_2_bytes + ge_4_bytes + ge_8_bytes) * 2 + unsigned
    }
}

pub(crate) trait MtableLookupEntryEncode<T: Add<T, Output = T> + Mul<T, Output = T>> {
    fn bn_to_t(_v: &BigUint) -> T;

    fn encode_stack_read(eid: T, emid: T, sp: T, vtype: T, value: T) -> T {
        eid * Self::bn_to_t(&EID_SHIFT)
            + emid * Self::bn_to_t(&EMID_SHIFT)
            + sp * Self::bn_to_t(&OFFSET_SHIFT)
            + Self::bn_to_t(&LOC_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(LocationType::Stack as u64))
            + Self::bn_to_t(&ACCESS_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(AccessType::Read as u64))
            + vtype * Self::bn_to_t(&VAR_TYPE_SHIFT)
            + value
    }

    fn encode_stack_write(eid: T, emid: T, sp: T, vtype: T, value: T) -> T {
        eid * Self::bn_to_t(&EID_SHIFT)
            + emid * Self::bn_to_t(&EMID_SHIFT)
            + sp * Self::bn_to_t(&OFFSET_SHIFT)
            + Self::bn_to_t(&LOC_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(LocationType::Stack as u64))
            + Self::bn_to_t(&ACCESS_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(AccessType::Write as u64))
            + vtype * Self::bn_to_t(&VAR_TYPE_SHIFT)
            + value
    }

    fn encode_memory_load(eid: T, emid: T, mmid: T, address: T, vtype: T, block_value: T) -> T {
        eid * Self::bn_to_t(&EID_SHIFT)
            + emid * Self::bn_to_t(&EMID_SHIFT)
            + mmid * Self::bn_to_t(&MMID_SHIFT)
            + address * Self::bn_to_t(&OFFSET_SHIFT)
            + Self::bn_to_t(&LOC_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(LocationType::Heap as u64))
            + Self::bn_to_t(&ACCESS_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(AccessType::Read as u64))
            + vtype * Self::bn_to_t(&VAR_TYPE_SHIFT)
            + block_value
    }

    fn encode_memory_store(eid: T, emid: T, mmid: T, address: T, vtype: T, block_value: T) -> T {
        eid * Self::bn_to_t(&EID_SHIFT)
            + emid * Self::bn_to_t(&EMID_SHIFT)
            + mmid * Self::bn_to_t(&MMID_SHIFT)
            + address * Self::bn_to_t(&OFFSET_SHIFT)
            + Self::bn_to_t(&LOC_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(LocationType::Heap as u64))
            + Self::bn_to_t(&ACCESS_TYPE_SHIFT)
                * Self::bn_to_t(&BigUint::from(AccessType::Write as u64))
            + vtype * Self::bn_to_t(&VAR_TYPE_SHIFT)
            + block_value
    }
}

impl<F: FieldExt> MtableLookupEntryEncode<BigUint> for MemoryTableConfig<F> {
    fn bn_to_t(v: &BigUint) -> BigUint {
        v.clone()
    }
}
