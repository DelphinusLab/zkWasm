use std::vec;

use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, u64::U64Config, Context},
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
    itable::{OpcodeClass, ShiftOp, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

const N32: u64 = 32;

pub struct BinShiftOpConfig<F: FieldExt> {
    left: U64Config<F>,
    right: U64Config<F>,
    shift: TValueConfig<F>,
    right_div_n: Column<Advice>,
    pow_of_shift: Column<Advice>,
    aux: Column<Advice>,
    aux2: Column<Advice>,
    res: U64Config<F>,
    enable: Column<Advice>,
    is_shl: Column<Advice>,
    is_shr_u: Column<Advice>,
    vtype: Column<Advice>,
    vtype_len_bitmask: [Column<Advice>; 4],
}

pub struct BinShiftOpConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinShiftOpConfigBuilder {
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
        let is_shl = cols.next().unwrap();
        let is_shr_u = cols.next().unwrap();

        let vtype = cols.next().unwrap();
        let vtype_len_bitmask = [0; 4].map(|_| cols.next().unwrap());

        let left = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        let shift = TValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right_div_n = cols.next().unwrap();
        let pow_of_shift = cols.next().unwrap();

        let aux = cols.next().unwrap();
        let aux2 = cols.next().unwrap();
        let res = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("bin is add or shl or shr_u", |meta| {
            macro_rules! op_selector {
                ($select_op:ident) => {
                    curr!(meta, $select_op)
                        * (curr!(meta, $select_op) - constant_from!(1))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                };
            }

            vec![
                op_selector!(is_shl),
                op_selector!(is_shr_u),
                (curr!(meta, is_shl) + curr!(meta, is_shr_u) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("vtype bits", |meta| {
            vtype_len_bitmask
                .iter()
                .map(|c| {
                    curr!(meta, *c)
                        * (curr!(meta, *c) - constant_from!(1))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("vtype sum one", |meta| {
            vec![
                (vtype_len_bitmask
                    .iter()
                    .map(|c| curr!(meta, *c))
                    .reduce(|acc, c| acc + c)
                    .unwrap()
                    - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        mtable.configure_stack_read_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, right.value),
        );

        mtable.configure_stack_read_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, left.value),
        );

        mtable.configure_stack_write_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, res.value),
        );

        rtable.configure_pow_2_lookup(
            meta,
            "shift pow 2 lookup",
            |meta| (curr!(meta, pow_of_shift), curr!(meta, shift.value.value)),
            |meta| curr!(meta, opcode_bit) * enable(meta),
        );

        meta.create_gate("shift is right mod 32", |meta| {
            vec![
                (curr!(meta, right_div_n) * constant_from!(N32) + curr!(meta, shift.value.value)
                    - curr!(meta, right.value))
                    * curr!(meta, vtype_len_bitmask[2])
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("shr_u result", |meta| {
            vec![
                (curr!(meta, aux) + curr!(meta, res.value) * curr!(meta, pow_of_shift)
                    - curr!(meta, left.value))
                    * curr!(meta, is_shr_u)
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        meta.create_gate("shl result", |meta| {
            vec![
                (curr!(meta, left.value) * curr!(meta, pow_of_shift) - curr!(meta, aux))
                    * curr!(meta, is_shl)
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, aux2) * constant!(F::from(1 << N32)) + curr!(meta, res.value)
                    - curr!(meta, aux))
                    * curr!(meta, is_shl)
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        // TODO: optimize them by merge the vtype cols into one
        meta.create_gate("op bin vtype constraints", |meta| {
            vec![
                (curr!(meta, vtype)
                    - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::I8)
                    - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::I16)
                    - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::I32)
                    - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::I64))
                    * (curr!(meta, vtype)
                        - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::U8)
                        - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::U16)
                        - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::U32)
                        - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::U64))
                    * enable(meta)
                    * curr!(meta, opcode_bit),
            ]
        });

        Box::new(BinShiftOpConfig {
            is_shl,
            is_shr_u,
            vtype_len_bitmask,
            enable: opcode_bit,
            left,
            right,
            right_div_n,
            shift,
            pow_of_shift,
            aux,
            aux2,
            res,
            vtype,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinShiftOpConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinShift as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.is_shl)
            * constant!(bn_to_field(
                &(BigUint::from(ShiftOp::Shl as u64) << OPCODE_ARG0_SHIFT)
            ))
            + curr!(meta, self.is_shr_u)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::UnsignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + curr!(meta, self.vtype)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT))))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::one()) * curr!(meta, self.enable)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BinShift
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::I32BinShiftOp {
                class,
                left,
                right,
                value,
            } => {
                let left = left as u32 as u64;
                let right = right as u32 as u64;

                ctx.region.assign_advice(
                    || "bin op i32 vtype",
                    self.vtype,
                    ctx.offset,
                    || Ok(F::from(VarType::I32 as u64)),
                )?;
                ctx.region.assign_advice(
                    || "bin op i32 vtype len bit",
                    self.vtype_len_bitmask[2],
                    ctx.offset,
                    || Ok(F::one()),
                )?;
                self.left.assign(ctx, left)?;
                self.right.assign(ctx, right)?;
                self.res.assign(ctx, value as u32 as u64)?;

                let shift = (right as u32 as u64) % N32;
                self.shift.assign(ctx, VarType::U32, shift)?;

                ctx.region.assign_advice(
                    || "right div n",
                    self.right_div_n,
                    ctx.offset,
                    || Ok(F::from(right / N32)),
                )?;

                ctx.region.assign_advice(
                    || "pow of shift",
                    self.pow_of_shift,
                    ctx.offset,
                    || Ok(F::from(1 << shift)),
                )?;

                match class {
                    ShiftOp::Shl => {
                        ctx.region.assign_advice(
                            || "bin op i32 shl",
                            self.is_shl,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;

                        ctx.region.assign_advice(
                            || "shl without module",
                            self.aux,
                            ctx.offset,
                            || Ok(F::from(left) * F::from(1 << shift)),
                        )?;

                        ctx.region.assign_advice(
                            || "quotient",
                            self.aux2,
                            ctx.offset,
                            || Ok(F::from(left * (1 << shift) / (1 << 32))),
                        )?;
                    }
                    ShiftOp::UnsignedShr => {
                        ctx.region.assign_advice(
                            || "bin op i32 shr_u",
                            self.is_shr_u,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;

                        ctx.region.assign_advice(
                            || "left cut",
                            self.aux,
                            ctx.offset,
                            || Ok(F::from(left % (1 << shift))),
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
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_i32_shr_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 3)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 35)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 33)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4294967295)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
