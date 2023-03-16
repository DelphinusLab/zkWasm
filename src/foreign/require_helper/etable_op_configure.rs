use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
    mtable::LocationType,
    step::StepInfo,
};

use crate::{
    circuits::{
        cell::{AllocatedU64Cell, AllocatedUnlimitedCell, CellExpression},
        etable::{
            allocator::{AllocatedMemoryTableLookupReadCell, EventTableCellAllocator},
            constraint_builder::ConstraintBuilder,
            EventTableCommonConfig, EventTableOpcodeConfig,
        },
        utils::{
            bn_to_field, step_status::StepStatus, table_entry::EventTableEntryWithMemoryInfo,
            Context,
        },
    },
    constant_from, constant_from_bn,
    foreign::{EventTableForeignCallConfigBuilder, InternalHostPluginBuilder},
};

pub struct ETableRequireHelperTableConfig<F: FieldExt> {
    plugin_index: usize,

    cond: AllocatedU64Cell<F>,
    cond_inv: AllocatedUnlimitedCell<F>,

    memory_table_lookup_read_stack: AllocatedMemoryTableLookupReadCell<F>,
}

pub struct ETableRequireHelperTableConfigBuilder {
    index: usize,
}

impl InternalHostPluginBuilder for ETableRequireHelperTableConfigBuilder {
    fn new(index: usize) -> Self {
        Self { index }
    }
}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for ETableRequireHelperTableConfigBuilder {
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = allocator.alloc_u64_cell();
        let cond_inv = allocator.alloc_unlimited_cell();

        constraint_builder.push(
            "require: cond is not zero",
            Box::new(move |meta| vec![(cond.expr(meta) * cond_inv.expr(meta) - constant_from!(1))]),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_read_stack = allocator.alloc_memory_table_lookup_read_cell(
            "wasm input stack read",
            constraint_builder,
            eid,
            move |_| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| cond.expr(meta),
            move |_| constant_from!(1),
        );

        Box::new(ETableRequireHelperTableConfig {
            plugin_index: self.index,
            cond,
            cond_inv,
            memory_table_lookup_read_stack,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableRequireHelperTableConfig<F> {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from_bn!(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + self.plugin_index as u64)
                << OPCODE_CLASS_SHIFT)
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::CallHost {
                args,
                ..
            } => {
                let cond = args[0];

                self.cond.assign(ctx, cond)?;
                self.cond_inv
                    .assign(ctx, F::from(cond).invert().unwrap_or(F::zero()))?;
                self.memory_table_lookup_read_stack.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    cond,
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }
}
