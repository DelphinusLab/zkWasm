use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, Context},
    },
    constant, constant_from, curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};
use std::marker::PhantomData;

pub struct LocalGetConfig<F: FieldExt> {
    offset: Column<Advice>,
    tvalue: TValueConfig<F>,
    _mark: PhantomData<F>,
}

pub struct LocalGetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalGetConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let offset = cols.next().unwrap();
        let tvalue = TValueConfig::configure(meta, cols, rtable, |meta| curr!(meta, opcode_bit));

        rtable.configure_in_common_range(meta, "localget offset range", |meta| {
            curr!(meta, opcode_bit) * curr!(meta, offset)
        });

        mtable.configure_stack_read_in_table(
            "local get mlookup 1",
            meta,
            |meta| curr!(meta, opcode_bit),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, tvalue.vtype),
            |meta| curr!(meta, tvalue.value.value),
        );

        mtable.configure_stack_write_in_table(
            "local get mlookup 2",
            meta,
            |meta| curr!(meta, opcode_bit),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, tvalue.vtype),
            |meta| curr!(meta, tvalue.value.value),
        );

        Box::new(LocalGetConfig {
            offset,
            tvalue,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalGetConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::LocalGet as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.tvalue.vtype)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.offset)
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from!(1u64)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::LocalGet
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::GetLocal {
                vtype,
                depth,
                value,
            } => {
                ctx.region.assign_advice(
                    || "op_const offset",
                    self.offset,
                    ctx.offset,
                    || Ok(F::from(depth as u64)),
                )?;

                self.tvalue.assign(ctx, vtype, value)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
