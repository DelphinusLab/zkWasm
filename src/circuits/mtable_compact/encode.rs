use num_bigint::BigUint;
use specs::{
    encode::{memory_table::encode_memory_table_entry, FromBn},
    mtable::{AccessType, LocationType},
};

pub(crate) struct MemoryTableLookupEncode;

impl MemoryTableLookupEncode {
    pub(crate) fn encode_stack_read<T: FromBn>(eid: T, emid: T, sp: T, vtype: T, value: T) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            sp,
            T::from_bn(&BigUint::from(LocationType::Stack as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_stack_write<T: FromBn>(eid: T, emid: T, sp: T, vtype: T, value: T) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            sp,
            T::from_bn(&BigUint::from(LocationType::Stack as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_memory_load<T: FromBn>(
        eid: T,
        emid: T,
        address: T,
        vtype: T,
        block_value: T,
    ) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            address,
            T::from_bn(&BigUint::from(LocationType::Heap as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            block_value,
        )
    }

    pub(crate) fn encode_memory_store<T: FromBn>(
        eid: T,
        emid: T,
        address: T,
        vtype: T,
        block_value: T,
    ) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            address,
            T::from_bn(&BigUint::from(LocationType::Heap as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            block_value,
        )
    }

    pub(crate) fn encode_global_get<T: FromBn>(
        eid: T,
        emid: T,
        address: T,
        vtype: T,
        value: T,
    ) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            address,
            T::from_bn(&BigUint::from(LocationType::Global as u64)),
            T::from_bn(&BigUint::from(AccessType::Read.into_index())),
            vtype,
            value,
        )
    }

    pub(crate) fn encode_global_set<T: FromBn>(
        eid: T,
        emid: T,
        address: T,
        vtype: T,
        value: T,
    ) -> T {
        encode_memory_table_entry(
            eid,
            emid,
            address,
            T::from_bn(&BigUint::from(LocationType::Global as u64)),
            T::from_bn(&BigUint::from(AccessType::Write.into_index())),
            vtype,
            value,
        )
    }
}
