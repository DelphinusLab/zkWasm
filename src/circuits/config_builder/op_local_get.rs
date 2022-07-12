use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        utils::bn_to_field,
    },
    constant, constant_from, curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::itable::{OpcodeClass, OPCODE_CLASS_SHIFT};
use std::marker::PhantomData;

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
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
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
            |_meta| constant_from!(1u64),
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
            |_meta| constant_from!(2u64),
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
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::LocalGet as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.offset)
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from!(1u64)
    }
}
