use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntryWithMemoryInfo;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::step::StepInfo;

use crate::circuits::cell::AllocatedU64Cell;
use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::cell::CellExpression;
use crate::circuits::etable::allocator::AllocatedMemoryTableLookupReadCell;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::Context;
use crate::constant_from;
use crate::constant_from_bn;
use crate::foreign::EventTableForeignCallConfigBuilder;
use crate::foreign::InternalHostPluginBuilder;

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
        _lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
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
            "require stack read",
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
            StepInfo::CallHost { args, .. } => {
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
