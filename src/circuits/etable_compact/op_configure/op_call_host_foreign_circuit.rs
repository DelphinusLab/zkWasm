use super::*;
use crate::{
    circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context},
    constant,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    encode::opcode::encode_call_host,
    etable::EventTableEntry,
    external_host_call_table::{encode::encode_host_call_entry, ExternalHostCallSignature},
    mtable::VarType,
    step::StepInfo,
};

pub struct ExternalCallHostCircuitConfig {
    op: CommonRangeCell,
    value: U64Cell,
    value_is_ret: BitCell,
    stack_rw_lookup: MTableLookupCell,
    external_host_call_lookup: ExternalHostCallTableLookupCell,
}

pub struct ExternalCallHostCircuitConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ExternalCallHostCircuitConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let index = common.external_host_index_cell();
        let op = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let value_is_ret = common.alloc_bit_value();

        let stack_rw_lookup = common.alloc_mtable_lookup();
        let external_host_call_lookup = common.alloc_external_host_call_table_lookup();

        constraint_builder.push(
            "external host call lookup",
            Box::new(move |meta| {
                vec![
                    external_host_call_lookup.clone().expr(meta)
                        - encode_host_call_entry(
                            index.expr(meta),
                            op.expr(meta),
                            value_is_ret.expr(meta),
                            value.expr(meta),
                        ),
                ]
            }),
        );

        Box::new(ExternalCallHostCircuitConfig {
            op,
            value,
            value_is_ret,
            stack_rw_lookup,
            external_host_call_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ExternalCallHostCircuitConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call_host(self.op.expr(meta), self.value_is_ret.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::ExternalHostCall { op, value, sig } => {
                self.op.assign(ctx, F::from(*op as u64))?;
                self.value.assign(ctx, value.unwrap())?;
                self.value_is_ret.assign(ctx, sig.is_ret())?;
                self.external_host_call_lookup.assign(
                    ctx,
                    &encode_host_call_entry(
                        BigUint::from(step_info.current_external_host_call_index),
                        BigUint::from(*op as u64),
                        BigUint::from(sig.is_ret() as u64),
                        BigUint::from(value.unwrap()),
                    ),
                )?;

                match sig {
                    ExternalHostCallSignature::Argument => {
                        self.stack_rw_lookup.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_read(
                                BigUint::from(step_info.current.eid),
                                BigUint::from(1 as u64),
                                BigUint::from(step_info.current.sp + 1),
                                BigUint::from(VarType::I64 as u64),
                                BigUint::from(value.unwrap()),
                            ),
                        )?;
                    }
                    ExternalHostCallSignature::Return => {
                        self.stack_rw_lookup.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_write(
                                BigUint::from(step_info.current.eid),
                                BigUint::from(1 as u64),
                                BigUint::from(step_info.current.sp),
                                BigUint::from(VarType::I64 as u64),
                                BigUint::from(value.unwrap()),
                            ),
                        )?;
                    }
                }

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(
                (constant_from!(1) - self.value_is_ret.expr(meta))
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(1),
                        common_config.sp(meta) + constant_from!(1),
                        constant_from!(VarType::I64 as u64),
                        self.value.expr(meta),
                    )
                    + self.value_is_ret.expr(meta)
                        * MemoryTableLookupEncode::encode_stack_write(
                            common_config.eid(meta),
                            constant_from!(1),
                            common_config.sp(meta),
                            constant_from!(VarType::I64 as u64),
                            self.value.expr(meta),
                        ),
            ),
            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            self.value_is_ret.expr(meta) * constant!(-F::one())
                + (constant_from!(1) - self.value_is_ret.expr(meta)),
        )
    }

    fn external_host_call_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> bool {
        true
    }
}
