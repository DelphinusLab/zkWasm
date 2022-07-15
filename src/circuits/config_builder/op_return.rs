use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, Context},
    },
    constant, constant_from, curr, next,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

pub struct ReturnConfig<F: FieldExt> {
    drop: Column<Advice>,
    keep: Column<Advice>,
    tvalue: TValueConfig<F>,
}

pub struct ReturnConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ReturnConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let drop = cols.next().unwrap();
        let keep = cols.next().unwrap();
        let tvalue = TValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta) * curr!(meta, keep)
        });

        meta.create_gate("keep is bit", |meta| {
            vec![curr!(meta, keep) * (curr!(meta, keep) - constant_from!(1)) * enable(meta)]
        });

        rtable.configure_in_common_range(meta, "return drop range", |meta| {
            curr!(meta, opcode_bit) * curr!(meta, drop) * enable(meta)
        });

        mtable.configure_stack_read_in_table(
            "return mlookup 1",
            meta,
            |meta| curr!(meta, opcode_bit) * curr!(meta, keep) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, tvalue.vtype),
            |meta| curr!(meta, tvalue.value.value),
        );

        mtable.configure_stack_write_in_table(
            "return mlookup 2",
            meta,
            |meta| curr!(meta, opcode_bit) * curr!(meta, keep) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.sp) - curr!(meta, drop),
            |meta| curr!(meta, tvalue.vtype),
            |meta| curr!(meta, tvalue.value.value),
        );

        jtable.configure_in_table(
            meta,
            |meta| curr!(meta, opcode_bit) * curr!(meta, keep) * enable(meta),
            |meta| curr!(meta, common.last_jump_eid),
            |meta| next!(meta, common.last_jump_eid),
            |meta| next!(meta, common.moid),
            |meta| next!(meta, common.fid),
            |meta| next!(meta, common.iid),
        );

        Box::new(ReturnConfig { drop, keep, tvalue })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ReturnConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.drop)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.keep)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + curr!(meta, self.tvalue.vtype)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        curr!(meta, self.keep) - curr!(meta, self.drop)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Return
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::Return {
                drop,
                keep,
                keep_values,
                ..
            } => {
                assert!(keep.len() <= 1);
                if keep.len() > 0 {
                    self.tvalue
                        .assign(ctx, VarType::from(keep[0]), keep_values[0])?;
                }

                ctx.region.assign_advice(
                    || "opcode return drop",
                    self.drop,
                    ctx.offset,
                    || Ok(F::from(*drop as u64)),
                )?;

                ctx.region.assign_advice(
                    || "opcode return keep",
                    self.keep,
                    ctx.offset,
                    || Ok(if keep.is_empty() { F::zero() } else { F::one() }),
                )?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
