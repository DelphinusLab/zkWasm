use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        utils::{bn_to_field, Context}, rtable::RangeTableConfig,
    },
    constant, curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
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
        _rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
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

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Drop
    }

    fn assign(&self, _ctx: &mut Context<'_, F>, _entry: &EventTableEntry) -> Result<(), Error> {
        Ok(())
    }
}
