use crate::{
    circuits::{
        etable::{EventTableOpcodeConfig, EventTableOpcodeConfigBuilder, EventTableCommonConfig},
        itable::InstTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        utils::bn_to_field,
    },
    constant, curr,
    spec::itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};
use num_bigint::BigUint;
use std::marker::PhantomData;

pub struct DropConfig<F: FieldExt> {
    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct DropConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for DropConfigBuilder {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        _common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        _cols: &mut impl Iterator<Item = Column<Advice>>,
        _itable: &InstTableConfig<F>,
        _mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        Box::new(DropConfig {
            enable: opcode_bit,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for DropConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT)
        )) * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(-F::one()) * curr!(meta, self.enable)
    }
}
