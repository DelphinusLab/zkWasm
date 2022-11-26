use crate::{
    circuits::{
        etable_compact::{
            op_configure::{
                ConstraintBuilder, EventTableCellAllocator, EventTableOpcodeConfig,
                MTableLookupCell, U64Cell, UnlimitedCell,
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

pub struct RequireForeignCallInfo {}
impl ForeignCallInfo for RequireForeignCallInfo {
    fn call_id(&self) -> usize {
        OpcodeClass::ForeignPluginStart as usize + HostPlugin::Require as usize
    }
}

pub struct ETableRequireHelperTableConfig {
    cond: U64Cell,
    cond_inv: UnlimitedCell,
    cond_lookup: MTableLookupCell,
}

pub struct ETableRequireHelperTableConfigBuilder {}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for ETableRequireHelperTableConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        _info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = common.alloc_u64();
        let cond_inv = common.alloc_unlimited_value();
        let cond_lookup = common.alloc_mtable_lookup();

        constraint_builder.push(
            "require: cond is not zero",
            Box::new(move |meta| vec![(cond.expr(meta) * cond_inv.expr(meta) - constant_from!(1))]),
        );

        Box::new(ETableRequireHelperTableConfig {
            cond,
            cond_inv,
            cond_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableRequireHelperTableConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + HostPlugin::Require as u64)
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
                assert_eq!(*plugin, HostPlugin::Require);

                let cond = args[0];

                self.cond.assign(ctx, cond)?;
                self.cond_inv
                    .assign(ctx, F::from(cond).invert().unwrap_or(F::zero()))?;
                self.cond_lookup.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u64),
                        BigUint::from(cond),
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
                self.cond.expr(meta),
            )),
            _ => None,
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        unreachable!()
    }
}
