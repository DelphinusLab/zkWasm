use crate::{
    circuits::{
        etable_compact::{
            op_configure::{
                ConstraintBuilder, EventTableCellAllocator, EventTableOpcodeConfig,
                MTableLookupCell, U64Cell,
            },
            EventTableCommonConfig, MLookupItem, StepStatus,
        },
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant, constant_from,
    foreign::{EventTableForeignCallConfigBuilder, ForeignCallInfo},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{host_function::HostPlugin, mtable::VarType};

pub struct PrintForeignCallInfo {}
impl ForeignCallInfo for PrintForeignCallInfo {
    fn call_id(&self) -> usize {
        OpcodeClass::ForeignPluginStart as usize + HostPlugin::Print as usize
    }
}

pub struct ETablePrintHelperTableConfig {
    value: U64Cell,
    value_lookup: MTableLookupCell,
}

pub struct ETablePrintHelperTableConfigBuilder {}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for ETablePrintHelperTableConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
        _info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = common.alloc_u64();
        let value_lookup = common.alloc_mtable_lookup();

        // No constraint

        Box::new(ETablePrintHelperTableConfig {
            value,
            value_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETablePrintHelperTableConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + HostPlugin::Print as u64)
                << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::CallHost { plugin, args, .. } => {
                assert_eq!(*plugin, HostPlugin::Print);

                let value = args[0];

                self.value.assign(ctx, value)?;
                self.value_lookup.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u64),
                        BigUint::from(value),
                    ),
                )?;

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        _entry: &EventTableEntry,
    ) -> u64 {
        1
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
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
                constant_from!(VarType::I32),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        unreachable!()
    }
}
