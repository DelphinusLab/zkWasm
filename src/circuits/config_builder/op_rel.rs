use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, u64::U64Config, Context},
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
    itable::{OpcodeClass, RelOp, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};
use std::vec;

pub struct RelOpConfig<F: FieldExt> {
    left: U64Config<F>,
    right: U64Config<F>,
    res: U64Config<F>,
    vtype: Column<Advice>,
    enable: Column<Advice>,
    is_eq: Column<Advice>,
    is_ne: Column<Advice>,
}

pub struct RelOpConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for RelOpConfigBuilder {
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
        let is_eq = cols.next().unwrap();
        let is_ne = cols.next().unwrap();
        let vtype = cols.next().unwrap();

        let left = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let res = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("rel is eq or ne", |meta| {
            vec![
                curr!(meta, is_eq)
                    * (curr!(meta, is_eq) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                curr!(meta, is_ne)
                    * (curr!(meta, is_ne) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, is_eq) + curr!(meta, is_ne) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("res is bool", |meta| {
            vec![
                curr!(meta, res.value)
                    * (curr!(meta, res.value) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        mtable.configure_stack_read_in_table(
            "rel mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, right.value),
        );

        mtable.configure_stack_read_in_table(
            "rel mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, left.value),
        );

        mtable.configure_stack_write_in_table(
            "rel mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |_| constant_from!(VarType::I32),
            |meta| curr!(meta, res.value),
        );

        Box::new(RelOpConfig {
            res,
            left,
            right,
            vtype,
            enable: opcode_bit,
            is_eq,
            is_ne,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for RelOpConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Rel as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.is_eq)
            * constant!(bn_to_field(
                &(BigUint::from(RelOp::Eq as u64) << OPCODE_ARG0_SHIFT)
            ))
            + curr!(meta, self.is_ne)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::Ne as u64) << OPCODE_ARG0_SHIFT)
                ))
            + curr!(meta, self.vtype)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT))))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::one()) * curr!(meta, self.enable)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Rel
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::I32Comp {
                class,
                left,
                right,
                value,
            } => {
                self.left.assign(ctx, left as u32 as u64)?;
                self.right.assign(ctx, right as u32 as u64)?;
                self.res.assign(ctx, value as u32 as u64)?;

                ctx.region.assign_advice(
                    || "bin op vytpe",
                    self.vtype,
                    ctx.offset,
                    || Ok((VarType::I32 as u64).into()),
                )?;

                match class {
                    RelOp::Eq => {
                        ctx.region.assign_advice(
                            || "rel op i32 eq",
                            self.is_eq,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                    }
                    RelOp::Ne => {
                        ctx.region.assign_advice(
                            || "rel op i32 ne",
                            self.is_ne,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                    }
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::test_circuit_builder::run_test_circuit,
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_i32_ne() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 2)
                      (i32.ne)
                      (drop)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    #[test]
    fn test_i32_eq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.eq)
                      (drop)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
