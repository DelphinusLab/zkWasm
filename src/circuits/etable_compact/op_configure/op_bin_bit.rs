use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        rtable::{BinOp, pow_table_encode},
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::itable::{BitOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};


pub struct BinBitConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: U64Cell,
    op: U4BopCell,
    op_class: U64OnU8Cell,
    op_lookup: PowTableLookupCell,
    vtype: CommonRangeCell,

    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}


pub struct BinBitConfigBuilder {}


impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinBitConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_u64();
        let op = common.alloc_u4_bop();
        let op_class = common.alloc_u64_on_u8();
        let op_lookup = common.alloc_pow_table_lookup();
        let vtype = common.alloc_common_range_value();

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "binbit op class",
            Box::new(move |meta| {
                vec![op_lookup.expr(meta) - pow_table_encode(op.expr(meta), constant_from!(12) * op_class.expr(meta))]
            })
        ); 

        constraint_builder.push(
            "binbit eq constraints",
            Box::new(move |meta| {
                vec![
                    op.eq_constraint(meta)
                ]
            }),
        );

        // limit the power to be u16 size
        constraint_builder.push(
            "binbit op_class range",
            Box::new(move |meta| {
                vec![
                    op_class.u8_expr(meta, 2)   
                    + op_class.u8_expr(meta, 3)   
                    + op_class.u8_expr(meta, 4)   
                    + op_class.u8_expr(meta, 5)   
                    + op_class.u8_expr(meta, 6)   
                    + op_class.u8_expr(meta, 7)   
                ]
            }),
        );

        Box::new(BinBitConfig {
            lhs,
            rhs,
            res,
            op,
            op_class,
            op_lookup,
            vtype,

            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })

    }
}


impl<F: FieldExt> EventTableOpcodeConfig<F> for BinBitConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinBit as u64) << OPCODE_CLASS_SHIFT)
        )) + self.op_class.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)
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
        let (class, vtype, left, right, value ) =
            match entry.step_info {
                StepInfo::I32BinBitOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    let vtype = VarType::I32;
                    let left = left as u32 as u64;
                    let right = right as u32 as u64;
                    let value = value as u32 as u64;
                    (class, vtype, left, right, value)
                },
                StepInfo::I64BinBitOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    let vtype = VarType::I64;
                    let left = left as u64;
                    let right = right as u64;
                    let value = value as u64;
                    (class, vtype, left, right, value)
                },
                _ => unreachable!(),
            };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;


        match class {
            specs::itable::BitOp::And => {
                // TODO
            },
            specs::itable::BitOp::Or => {
                // TODO
            },
            specs::itable::BitOp::Xor => {
                self.op_class.assign(ctx, BinOp::Xor as u64)?;
                self.op.assign(ctx, bn_to_field(&(BigUint::from(1u64) << (BinOp::Xor as usize * 12))))?;
                self.op_lookup.assign(ctx, (BinOp::Xor as u64) * 12)?;
            },
            specs::itable::BitOp::Not => {
                unimplemented!()
            },
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
        OpcodeClass::BinBit
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
    fn test_i32_xor() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      i32.xor
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_xor1() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 21)
                      (i32.const 31)
                      i32.xor
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

}

