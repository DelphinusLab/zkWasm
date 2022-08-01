use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, bytes8::Bytes8Config, Context},
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
    left: Bytes8Config<F>,
    right: Bytes8Config<F>,
    res: Column<Advice>,
    vtype: Column<Advice>,
    enable: Column<Advice>,
    is_eq: Column<Advice>,
    is_ne: Column<Advice>,
    is_gt: Column<Advice>,
    is_ge: Column<Advice>,
    is_signed: Column<Advice>,
    is_same: Column<Advice>,
    eq_bytes: [Column<Advice>; 8],
    gt_bytes: [Column<Advice>; 8],
    lt_bytes: [Column<Advice>; 8],
    inv: Column<Advice>,
}

pub struct RelOpConfigBuilder {}

impl RelOpConfigBuilder {
    fn _constraint_builder<F: FieldExt>(
        meta: &mut VirtualCells<F>,
        gt_bytes: &[Column<Advice>; 8],
        eq_bytes: &[Column<Advice>; 8],
        lt_bytes: &[Column<Advice>; 8],
        res_gt: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
        res_eq: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
        res_lt: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
        position: usize,
    ) -> Expression<F> {
        curr!(meta, gt_bytes[position]) * res_gt(meta)
            + curr!(meta, lt_bytes[position]) * res_lt(meta)
            + curr!(meta, eq_bytes[position])
                * (if position == 7 {
                    res_eq(meta)
                } else {
                    Self::_constraint_builder(
                        meta,
                        gt_bytes,
                        eq_bytes,
                        lt_bytes,
                        res_gt,
                        res_eq,
                        res_lt,
                        position + 1,
                    )
                })
    }

    fn constraint_builder<F: FieldExt>(
        meta: &mut VirtualCells<F>,
        gt_bytes: &[Column<Advice>; 8],
        eq_bytes: &[Column<Advice>; 8],
        lt_bytes: &[Column<Advice>; 8],
        res_gt: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
        res_eq: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
        res_lt: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
    ) -> Expression<F> {
        Self::_constraint_builder(
            meta, gt_bytes, eq_bytes, lt_bytes, res_gt, res_eq, res_lt, 0,
        )
    }
}

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
        let is_gt = cols.next().unwrap();
        let is_ge = cols.next().unwrap();
        let is_signed = cols.next().unwrap();
        let inv = cols.next().unwrap();
        let is_same = cols.next().unwrap();
        let res = cols.next().unwrap();
        let vtype = cols.next().unwrap();

        let left = Bytes8Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right = Bytes8Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let eq_bytes = [(); 8].map(|_| cols.next().unwrap());
        let lt_bytes = [(); 8].map(|_| cols.next().unwrap());
        let gt_bytes = [(); 8].map(|_| cols.next().unwrap());

        meta.create_gate("rel is eq or ne or gt", |meta| {
            macro_rules! is_op {
                ($select_op:ident) => {
                    curr!(meta, $select_op)
                        * (curr!(meta, $select_op) - constant_from!(1))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                };
            }
            vec![
                is_op!(is_eq),
                is_op!(is_ne),
                is_op!(is_gt),
                is_op!(is_ge),
                (curr!(meta, is_eq) + curr!(meta, is_ne) + curr!(meta, is_gt) + curr!(meta, is_ge)
                    - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("rel inv same", |meta| {
            vec![
                ((curr!(meta, left.value) - curr!(meta, right.value)) * curr!(meta, inv)
                    + curr!(meta, is_same)
                    - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, left.value) - curr!(meta, right.value))
                    * curr!(meta, is_same)
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("eq or ne res constaints", |meta| {
            vec![
                curr!(meta, is_eq)
                    * (curr!(meta, res) - curr!(meta, is_same))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                curr!(meta, is_ne)
                    * (curr!(meta, res) + curr!(meta, is_same) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("is_signed is either zero or one", |meta| {
            vec![
                curr!(meta, is_signed)
                    * (constant_from!(1) - curr!(meta, is_signed))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("rel bytes select one", |meta| {
            (0..8usize)
                .map(|i| {
                    (curr!(meta, lt_bytes[i]) + curr!(meta, gt_bytes[i]) + curr!(meta, eq_bytes[i])
                        - constant_from!(1))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("eq bytes is either zero or one", |meta| {
            (0..8usize)
                .map(|i| {
                    (curr!(meta, eq_bytes[i]) * (curr!(meta, eq_bytes[i]) - constant_from!(1)))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("lt bytes is either zero or one", |meta| {
            (0..8usize)
                .map(|i| {
                    (curr!(meta, lt_bytes[i]) * (curr!(meta, lt_bytes[i]) - constant_from!(1)))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("gt bytes is either zero or one", |meta| {
            (0..8usize)
                .map(|i| {
                    (curr!(meta, gt_bytes[i]) * (curr!(meta, gt_bytes[i]) - constant_from!(1)))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("eq bytes constraints", |meta| {
            (0..8usize)
                .map(|i| {
                    curr!(meta, eq_bytes[i])
                        * (curr!(meta, left.bytes_le[i]) - curr!(meta, right.bytes_le[i]))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        for i in 0..8usize {
            rtable.configure_lt_lookup(
                meta,
                "lt byte lookup",
                |meta| {
                    (
                        curr!(meta, left.bytes_le[i]),
                        curr!(meta, right.bytes_le[i]),
                    )
                },
                |meta| curr!(meta, opcode_bit) * enable(meta) * curr!(meta, lt_bytes[i]),
            )
        }

        meta.create_gate("gt_u constraint", |meta| {
            vec![
                curr!(meta, opcode_bit)
                    * enable(meta)
                    * curr!(meta, is_gt)
                    * (constant_from!(1) - curr!(meta, is_signed))
                    * Self::constraint_builder(
                        meta,
                        &gt_bytes,
                        &eq_bytes,
                        &lt_bytes,
                        |meta| curr!(meta, res) - constant_from!(1),
                        |meta| curr!(meta, res),
                        |meta| curr!(meta, res),
                    ),
            ]
        });

        meta.create_gate("ge_u constraint", |meta| {
            vec![
                curr!(meta, opcode_bit)
                    * enable(meta)
                    * curr!(meta, is_ge)
                    * (constant_from!(1) - curr!(meta, is_signed))
                    * Self::constraint_builder(
                        meta,
                        &gt_bytes,
                        &eq_bytes,
                        &lt_bytes,
                        |meta| curr!(meta, res) - constant_from!(1),
                        |meta| curr!(meta, res) - constant_from!(1),
                        |meta| curr!(meta, res),
                    ),
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
            |meta| curr!(meta, res),
        );

        Box::new(RelOpConfig {
            res,
            left,
            right,
            vtype,
            enable: opcode_bit,
            is_eq,
            is_ne,
            is_gt,
            is_ge,
            is_signed,
            is_same,
            eq_bytes,
            gt_bytes,
            lt_bytes,
            inv,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for RelOpConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let subop_eq = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_eq)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::Eq as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ne = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_ne)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::Ne as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_gt_s = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_gt)
                * curr!(meta, self.is_signed)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedGt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_gt_u = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_gt)
                * (constant_from!(1) - curr!(meta, self.is_signed))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedGt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ge_s = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_ge)
                * curr!(meta, self.is_signed)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedGe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ge_u = |meta: &mut VirtualCells<F>| {
            curr!(meta, self.is_ge)
                * (constant_from!(1) - curr!(meta, self.is_signed))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedGe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop = |meta: &mut VirtualCells<F>| {
            subop_eq(meta)
                + subop_ne(meta)
                + subop_gt_s(meta)
                + subop_gt_u(meta)
                + subop_ge_s(meta)
                + subop_ge_u(meta)
        };

        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Rel as u64) << OPCODE_CLASS_SHIFT)
        )) + subop(meta)
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

                ctx.region.assign_advice(
                    || "rel res",
                    self.res,
                    ctx.offset,
                    || Ok((value as u64).into()),
                )?;

                ctx.region.assign_advice(
                    || "rel inv",
                    self.inv,
                    ctx.offset,
                    || {
                        Ok((F::from(left as u32 as u64) - F::from(right as u32 as u64))
                            .invert()
                            .unwrap_or(F::zero()))
                    },
                )?;

                ctx.region.assign_advice(
                    || "rel is_same",
                    self.is_same,
                    ctx.offset,
                    || Ok(if left == right { F::one() } else { F::zero() }),
                )?;

                ctx.region.assign_advice(
                    || "rel vytpe",
                    self.vtype,
                    ctx.offset,
                    || Ok((VarType::I32 as u64).into()),
                )?;

                let mut left_bytes = Vec::from((left as u32 as u64).to_le_bytes());
                left_bytes.resize(8, 0);
                let mut right_bytes = Vec::from((right as u32 as u64).to_le_bytes());
                right_bytes.resize(8, 0);
                for position in 0..8usize {
                    ctx.region.assign_advice(
                        || "gt bytes",
                        self.gt_bytes[position],
                        ctx.offset,
                        || {
                            Ok(F::from(
                                (left_bytes[position] > right_bytes[position]) as u64,
                            ))
                        },
                    )?;

                    ctx.region.assign_advice(
                        || "lt bytes",
                        self.lt_bytes[position],
                        ctx.offset,
                        || {
                            Ok(F::from(
                                (left_bytes[position] < right_bytes[position]) as u64,
                            ))
                        },
                    )?;

                    ctx.region.assign_advice(
                        || "eq bytes",
                        self.eq_bytes[position],
                        ctx.offset,
                        || {
                            Ok(F::from(
                                (left_bytes[position] == right_bytes[position]) as u64,
                            ))
                        },
                    )?;
                }

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
                    RelOp::SignedGt => {
                        ctx.region.assign_advice(
                            || "rel op i32 gt_s",
                            self.is_gt,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                        ctx.region.assign_advice(
                            || "rel op i32 signed",
                            self.is_signed,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;

                        // support constraint
                        unimplemented!();
                    }
                    RelOp::UnsignedGt => {
                        ctx.region.assign_advice(
                            || "rel op i32 gt_u",
                            self.is_gt,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                    }
                    RelOp::SignedGe => {
                        // support constraint
                        ctx.region.assign_advice(
                            || "rel op i32 ge_s",
                            self.is_ge,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                        ctx.region.assign_advice(
                            || "rel op i32 signed",
                            self.is_signed,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;

                        unimplemented!();
                    }
                    RelOp::UnsignedGe => {
                        ctx.region.assign_advice(
                            || "rel op i32 gt_u",
                            self.is_ge,
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
    use crate::test::test_circuit_builder::test_circuit_noexternal;

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

        test_circuit_noexternal(textual_repr).unwrap()
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

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_gt_s() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.gt_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_gt1_u() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.gt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_gt2_u() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      (i32.gt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_gt3_u() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 0)
                      (i32.gt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_ge_s() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.ge_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_ge_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.ge_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_ge_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      (i32.ge_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_ge_u_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 0)
                      (i32.ge_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
