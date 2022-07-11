use std::marker::PhantomData;

use crate::{
    constant, constant_from, curr,
    etable::{EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
    itable::InstTableConfig,
    jtable::JumpTableConfig,
    mtable::MemoryTableConfig,
    opcode::Opcode,
    utils::bn_to_field,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};
use num_bigint::BigUint;

pub struct LocalGetConfig<F: FieldExt> {
    offset: Column<Advice>,
    vtype: Column<Advice>,
    value: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct LocalGetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalGetConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &crate::etable::EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        itable: &InstTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let offset = cols.next().unwrap();
        let value = cols.next().unwrap();
        let vtype = cols.next().unwrap();

        mtable.configure_stack_read_in_table(
            "local get mlookup",
            "local get mlookup rev",
            meta,
            |meta| curr!(meta, opcode_bit),
            |meta| curr!(meta, common.eid),
            |meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value),
        );

        mtable.configure_stack_write_in_table(
            "local get mlookup",
            "local get mlookup rev",
            meta,
            |meta| curr!(meta, opcode_bit),
            |meta| curr!(meta, common.eid),
            |meta| constant_from!(2u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value),
        );

        Box::new(LocalGetConfig {
            offset,
            value,
            vtype,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalGetConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(&(BigUint::from(Opcode::LocalGet as u64) << 64)))
            + curr!(meta, self.offset)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from!(1u64)
    }
}
