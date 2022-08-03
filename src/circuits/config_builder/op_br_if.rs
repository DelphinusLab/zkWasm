use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, Context},
    },
    constant, constant_from, curr, fixed_curr, next,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

pub struct BrIfConfig<F: FieldExt> {
    drop: Column<Advice>,
    keep: Column<Advice>,
    keep_value: TValueConfig<F>,
    condition: Column<Advice>,
    condition_inv: Column<Advice>,
    dst_pc: Column<Fixed>,
    enable: Column<Advice>,
}

pub struct BrIfConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let drop = cols.next().unwrap();
        let keep = cols.next().unwrap();
        let keep_value = TValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta) * curr!(meta, keep)
        });
        let condition = cols.next().unwrap();
        let condition_inv = cols.next().unwrap();
        let dst_pc = meta.fixed_column();

        rtable.configure_in_u16_range(meta, "op br_if drop limit", |meta| {
            curr!(meta, drop) * curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("op br_if keep is bit", |meta| {
            vec![
                curr!(meta, keep)
                    * (curr!(meta, keep) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        mtable.configure_stack_read_in_table(
            "read cond",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |_meta| constant_from!(VarType::I32),
            |meta| curr!(meta, condition),
        );

        mtable.configure_stack_read_in_table(
            "read return value",
            meta,
            |meta| {
                curr!(meta, opcode_bit) * enable(meta) * curr!(meta, condition) * curr!(meta, keep)
            },
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, keep_value.vtype),
            |meta| curr!(meta, keep_value.value.value),
        );

        mtable.configure_stack_write_in_table(
            "write return value",
            meta,
            |meta| {
                curr!(meta, opcode_bit) * enable(meta) * curr!(meta, condition) * curr!(meta, keep)
            },
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3),
            |meta| curr!(meta, common.sp) + constant_from!(2) + curr!(meta, drop),
            |meta| curr!(meta, keep_value.vtype),
            |meta| curr!(meta, keep_value.value.value),
        );

        meta.create_gate("br pc jump", |meta| {
            vec![
                // if condition is non-zero
                curr!(meta, condition)
                    * (next!(meta, common.iid) - fixed_curr!(meta, dst_pc))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                // if condition is zero
                (curr!(meta, condition) * curr!(meta, condition_inv) - constant_from!(1))
                    * (next!(meta, common.iid) - curr!(meta, common.iid) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        Box::new(BrIfConfig {
            drop,
            keep,
            keep_value,
            dst_pc,
            condition,
            condition_inv,
            enable: opcode_bit,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfConfig<F> {
    fn handle_iid(&self) -> bool {
        true
    }

    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BrIf as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.drop)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.keep)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + fixed_curr!(meta, self.dst_pc))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(F::one())
            + (curr!(meta, self.condition) * curr!(meta, self.condition_inv))
                * (curr!(meta, self.drop)))
            * curr!(meta, self.enable)
    }

    fn extra_mops(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        ((curr!(meta, self.condition) * curr!(meta, self.condition_inv))
            * constant_from!(2)
            * curr!(meta, self.keep))
            * curr!(meta, self.enable)
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::BrIfNez {
                condition,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);
                if keep.len() > 0 {
                    self.keep_value
                        .assign(ctx, VarType::from(keep[0]), keep_values[0])?;
                }

                ctx.region.assign_advice(
                    || "opcode br if condition",
                    self.condition,
                    ctx.offset,
                    || Ok(F::from(*condition as u64)),
                )?;

                ctx.region.assign_advice(
                    || "br if condition inv",
                    self.condition_inv,
                    ctx.offset,
                    || Ok((F::from(*condition as u64)).invert().unwrap_or(F::zero())),
                )?;

                ctx.region.assign_advice(
                    || "opcode br if drop",
                    self.drop,
                    ctx.offset,
                    || Ok(F::from(*drop as u64)),
                )?;

                ctx.region.assign_advice(
                    || "opcode br if keep",
                    self.keep,
                    ctx.offset,
                    || Ok(if keep.is_empty() { F::zero() } else { F::one() }),
                )?;

                ctx.region.assign_fixed(
                    || "br dst_pc",
                    self.dst_pc,
                    ctx.offset,
                    || Ok(F::from(*dst_pc as u64)),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BrIf
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_br_if_trivial_nojump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 0)
                br_if 0
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_trivial_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 1)
                br_if 0
                (i32.const 0)
                drop
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_block_with_arg_do_not_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block (result i32)
                (i32.const 0)
                (i32.const 0)
                br_if 0
              )
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_block_with_arg_do_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block (result i32)
                (i32.const 0)
                (i32.const 1)
                br_if 0
              )
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_block_with_drop_do_not_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 0)
                  br_if 1
                  drop
                  drop
                )
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_block_with_drop_do_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 1)
                  br_if 1
                  drop
                  drop
                )
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
