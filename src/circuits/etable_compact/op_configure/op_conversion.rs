use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{ConversionOp, OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct ConversionConfig {
    value: U64Cell,
    value_type: CommonRangeCell,

    res: U64Cell,
    res_type: CommonRangeCell,

    is_i32_wrap_i64: BitCell,
    is_i64_extend_i32_u: BitCell,

    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct ConversionConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConversionConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = common.alloc_u64();
        let value_type = common.alloc_common_range_value();

        let res = common.alloc_u64();
        let res_type = common.alloc_common_range_value();

        let is_i32_wrap_i64 = common.alloc_bit_value();
        let is_i64_extend_i32_u = common.alloc_bit_value();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        todo!();

        Box::new(ConversionConfig {
            value,
            value_type,
            res,
            res_type,
            is_i32_wrap_i64,
            is_i64_extend_i32_u,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConversionConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let subop = self.is_i32_wrap_i64.expr(meta)
            * constant_from!(ConversionOp::I32WrapI64 as u64)
            + self.is_i64_extend_i32_u.expr(meta)
                * constant_from!(ConversionOp::I64ExtendUI32 as u64);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Conversion as u64) << OPCODE_CLASS_SHIFT)
        )) + subop * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (value, value_type, result, result_type) = match entry.step_info {
            StepInfo::I32WrapI64 { value, result } => {
                self.is_i32_wrap_i64.assign(ctx, true)?;

                (
                    value as u64,
                    VarType::I64,
                    result as u32 as u64,
                    VarType::I32,
                )
            }
            StepInfo::I64ExtendUI32 { value, result } => {
                self.is_i64_extend_i32_u.assign(ctx, true)?;

                (
                    value as u32 as u64,
                    VarType::I32,
                    result as u64,
                    VarType::I64,
                )
            }
            _ => unreachable!(),
        };

        self.value.assign(ctx, value)?;
        self.res.assign(ctx, result)?;
        self.value_type.assign(ctx, value_type as u16)?;
        self.res_type.assign(ctx, result_type as u16)?;

        self.lookup_stack_read.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(value_type as u16),
                BigUint::from(value),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(2u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(result_type as u16),
                BigUint::from(result),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Conversion
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
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
                self.value_type.expr(meta),
                self.value.expr(meta),
            )),

            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                self.res_type.expr(meta),
                self.res.expr(meta),
            )),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_i32_wrap_i64_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 0)
                      (i32.wrap_i64)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_extend_i32_u_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i64.extend_i32_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
