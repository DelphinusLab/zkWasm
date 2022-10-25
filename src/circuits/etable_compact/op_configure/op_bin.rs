use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use ark_std::One;
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::itable::{BinOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: U64Cell,
    overflow: BitCell,
    vtype: CommonRangeCell,
    is_add: BitCell,
    is_sub: BitCell,
    is_32bits: BitCell,
    is_64bits: BitCell,
    //TODO: add constraints between vtype and is_32bits, is_64bits
    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_u64();
        let overflow = common.alloc_bit_value();

        let vtype = common.alloc_common_range_value();

        let is_add = common.alloc_bit_value();
        let is_sub = common.alloc_bit_value();
        let is_32bits = common.alloc_bit_value();
        let is_64bits = common.alloc_bit_value();

        constraint_builder.push(
            "binop: is add or sub",
            Box::new(move |meta| vec![(is_add.expr(meta) + is_sub.expr(meta) - constant_from!(1))]),
        );

        constraint_builder.push(
            "binop: 32 or 64",
            Box::new(move |meta| {
                vec![(is_32bits.expr(meta) + is_64bits.expr(meta) - constant_from!(1))]
            }),
        );

        constraint_builder.push(
            "binop constraints",
            Box::new(move |meta| {
                let modules = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * is_32bits.expr(meta)
                    + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                        * is_64bits.expr(meta);

                vec![
                    (lhs.expr(meta) + rhs.expr(meta)
                        - res.expr(meta)
                        - overflow.expr(meta) * modules.clone())
                        * is_add.expr(meta),
                    (rhs.expr(meta) + res.expr(meta)
                        - lhs.expr(meta)
                        - overflow.expr(meta) * modules)
                        * is_sub.expr(meta),
                ]
            }),
        );

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(BinConfig {
            lhs,
            rhs,
            res,
            overflow,
            vtype,
            is_add,
            is_sub,
            is_32bits,
            is_64bits,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_add.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(BinOp::Add as u64) << OPCODE_ARG0_SHIFT)
            ))
            + self.is_sub.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::Sub as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.vtype.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, shift, left, right, value) = match &entry.step_info {
            StepInfo::I32BinOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = *left as u32 as u64;
                let right = *right as u32 as u64;
                let value = *value as u32 as u64;

                (class, vtype, 32, left, right, value)
            }

            StepInfo::I64BinOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I64;
                let left = *left as u64;
                let right = *right as u64;
                let value = *value as u64;

                (class, vtype, 64, left, right, value)
            }

            _ => unreachable!(),
        };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        match class {
            specs::itable::BinOp::Add => {
                self.is_add.assign(ctx, true)?;
                self.overflow.assign(
                    ctx,
                    (BigUint::from(left) + BigUint::from(right)) >> shift == BigUint::one(),
                )?;
            }
            specs::itable::BinOp::Sub => {
                self.is_sub.assign(ctx, true)?;
                self.overflow.assign(
                    ctx,
                    (BigUint::from(right) + BigUint::from(value)) >> shift == BigUint::one(),
                )?;
            }
            specs::itable::BinOp::Mul => {
                unimplemented!()
            }
        };

        match vtype {
            VarType::I32 => self.is_32bits.assign(ctx, true)?,
            VarType::I64 => self.is_64bits.assign(ctx, true)?,
        };

        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u16),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(value),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Bin
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(3))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.res.expr(meta),
            )),
            _ => None,
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_i32_add() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      i32.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_add_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 4294967295)
                      i32.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_add() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 1)
                      i64.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_add_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 18446744073709551615)
                      i64.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_sub() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      i32.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_sub_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      i32.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_sub() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 1)
                      i64.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_sub_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 0)
                      (i64.const 1)
                      i64.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
