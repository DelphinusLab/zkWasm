use self::allocator::*;
use self::constraint_builder::ConstraintBuilder;
use super::bit_table::BitTableConfig;
use super::cell::*;
use super::external_host_call_table::ExternalHostCallTableConfig;
use super::image_table::ImageTableConfig;
use super::jtable::JumpTableConfig;
use super::mtable::MemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::traits::ConfigureLookupTable;
use super::utils::step_status::StepStatus;
use super::utils::table_entry::EventTableEntryWithMemoryInfo;
use super::utils::Context;
use crate::circuits::etable::op_configure::op_bin::BinConfigBuilder;
use crate::circuits::etable::op_configure::op_bin_bit::BinBitConfigBuilder;
use crate::circuits::etable::op_configure::op_bin_shift::BinShiftConfigBuilder;
use crate::circuits::etable::op_configure::op_br::BrConfigBuilder;
use crate::circuits::etable::op_configure::op_br_if::BrIfConfigBuilder;
use crate::circuits::etable::op_configure::op_br_if_eqz::BrIfEqzConfigBuilder;
use crate::circuits::etable::op_configure::op_br_table::BrTableConfigBuilder;
use crate::circuits::etable::op_configure::op_call::CallConfigBuilder;
use crate::circuits::etable::op_configure::op_call_host_foreign_circuit::ExternalCallHostCircuitConfigBuilder;
use crate::circuits::etable::op_configure::op_call_indirect::CallIndirectConfigBuilder;
use crate::circuits::etable::op_configure::op_const::ConstConfigBuilder;
use crate::circuits::etable::op_configure::op_conversion::ConversionConfigBuilder;
use crate::circuits::etable::op_configure::op_drop::DropConfigBuilder;
use crate::circuits::etable::op_configure::op_global_get::GlobalGetConfigBuilder;
use crate::circuits::etable::op_configure::op_global_set::GlobalSetConfigBuilder;
use crate::circuits::etable::op_configure::op_load::LoadConfigBuilder;
use crate::circuits::etable::op_configure::op_local_get::LocalGetConfigBuilder;
use crate::circuits::etable::op_configure::op_local_set::LocalSetConfigBuilder;
use crate::circuits::etable::op_configure::op_local_tee::LocalTeeConfigBuilder;
use crate::circuits::etable::op_configure::op_memory_grow::MemoryGrowConfigBuilder;
use crate::circuits::etable::op_configure::op_memory_size::MemorySizeConfigBuilder;
use crate::circuits::etable::op_configure::op_rel::RelConfigBuilder;
use crate::circuits::etable::op_configure::op_return::ReturnConfigBuilder;
use crate::circuits::etable::op_configure::op_select::SelectConfigBuilder;
use crate::circuits::etable::op_configure::op_store::StoreConfigBuilder;
use crate::circuits::etable::op_configure::op_test::TestConfigBuilder;
use crate::circuits::etable::op_configure::op_unary::UnaryConfigBuilder;
use crate::constant_from;
use crate::fixed_curr;
use crate::foreign::context::etable_op_configure::ETableContextHelperTableConfigBuilder;
use crate::foreign::require_helper::etable_op_configure::ETableRequireHelperTableConfigBuilder;
use crate::foreign::wasm_input_helper::etable_op_configure::ETableWasmInputHelperTableConfigBuilder;
use crate::foreign::EventTableForeignCallConfigBuilder;
use crate::foreign::ForeignTableConfig;
use crate::foreign::InternalHostPluginBuilder;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::instruction_table::encode_instruction_table_entry;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::OpcodeClassPlain;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) mod assign;
mod op_configure;

pub(crate) mod allocator;
pub(crate) mod constraint_builder;

#[cfg(feature = "continuation")]
type AllocatedU32StateCell<F> = AllocatedU32PermutationCell<F>;
#[cfg(not(feature = "continuation"))]
type AllocatedU32StateCell<F> = AllocatedCommonRangeCell<F>;

pub(crate) const EVENT_TABLE_ENTRY_ROWS: i32 = 4;
pub(crate) const OP_CAPABILITY: usize = 32;

const FOREIGN_LOOKUP_CAPABILITY: usize = 6;

#[derive(Clone)]
pub struct EventTableCommonConfig<F: FieldExt> {
    enabled_cell: AllocatedBitCell<F>,
    ops: [AllocatedBitCell<F>; OP_CAPABILITY],

    rest_mops_cell: AllocatedCommonRangeCell<F>,
    rest_call_ops_cell: AllocatedUnlimitedCell<F>,
    rest_return_ops_cell: AllocatedUnlimitedCell<F>,
    pub(crate) input_index_cell: AllocatedCommonRangeCell<F>,
    pub(crate) context_input_index_cell: AllocatedCommonRangeCell<F>,
    pub(crate) context_output_index_cell: AllocatedCommonRangeCell<F>,
    external_host_call_index_cell: AllocatedCommonRangeCell<F>,
    pub(crate) sp_cell: AllocatedCommonRangeCell<F>,
    mpages_cell: AllocatedCommonRangeCell<F>,
    frame_id_cell: AllocatedU32StateCell<F>,
    pub(crate) eid_cell: AllocatedU32StateCell<F>,
    fid_cell: AllocatedCommonRangeCell<F>,
    iid_cell: AllocatedCommonRangeCell<F>,
    maximal_memory_pages_cell: AllocatedCommonRangeCell<F>,

    itable_lookup_cell: AllocatedUnlimitedCell<F>,
    brtable_lookup_cell: AllocatedUnlimitedCell<F>,
    jtable_lookup_cell: AllocatedUnlimitedCell<F>,
    is_returned_cell: AllocatedBitCell<F>,

    pow_table_lookup_modulus_cell: AllocatedUnlimitedCell<F>,
    pow_table_lookup_power_cell: AllocatedUnlimitedCell<F>,
    bit_table_lookup_cells: AllocatedBitTableLookupCells<F>,
    external_foreign_call_lookup_cell: AllocatedUnlimitedCell<F>,
}

pub(in crate::circuits::etable) trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        common: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error>;
    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        0
    }
    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn allocated_memory_pages_diff(
        &self,
        _meta: &mut VirtualCells<'_, F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn call_ops_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn call_ops(&self) -> u32 {
        0
    }
    fn return_ops_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn return_ops(&self) -> u32 {
        0
    }
    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_frame_id(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn next_fid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn input_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn is_host_public_input(&self, _entry: &EventTableEntry) -> bool {
        false
    }

    fn context_input_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn is_context_input_op(&self, _entry: &EventTableEntry) -> bool {
        false
    }
    fn context_output_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn is_context_output_op(&self, _entry: &EventTableEntry) -> bool {
        false
    }

    fn external_host_call_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn is_external_host_call(&self, _entry: &EventTableEntry) -> bool {
        false
    }
}

struct OpcodeConfig<F: FieldExt>(Box<dyn EventTableOpcodeConfig<F>>);

unsafe impl<F: FieldExt> Send for OpcodeConfig<F> {}
unsafe impl<F: FieldExt> Sync for OpcodeConfig<F> {}

#[derive(Clone)]
pub struct EventTableConfig<F: FieldExt> {
    pub step_sel: Column<Fixed>,
    pub common_config: EventTableCommonConfig<F>,
    op_configs: Arc<BTreeMap<OpcodeClassPlain, OpcodeConfig<F>>>,
}

impl<F: FieldExt> EventTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        (l_0, l_active, l_active_last): (Column<Fixed>, Column<Fixed>, Column<Fixed>),
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        image_table: &ImageTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
        bit_table: &BitTableConfig<F>,
        external_host_call_table: &ExternalHostCallTableConfig<F>,
        foreign_table_configs: &BTreeMap<&'static str, Box<dyn ForeignTableConfig<F>>>,
    ) -> EventTableConfig<F> {
        let step_sel = meta.fixed_column();

        let mut allocator = EventTableCellAllocator::new(
            meta,
            step_sel,
            (l_0, l_active, l_active_last),
            rtable,
            mtable,
            cols,
        );

        let ops = [0; OP_CAPABILITY].map(|_| allocator.alloc_bit_cell());
        let enabled_cell = allocator.alloc_bit_cell();

        let rest_mops_cell = allocator.alloc_common_range_cell();
        let rest_call_ops_cell = allocator.alloc_unlimited_cell();
        let rest_return_ops_cell = allocator.alloc_unlimited_cell();
        let input_index_cell = allocator.alloc_common_range_cell();
        let context_input_index_cell = allocator.alloc_common_range_cell();
        let context_output_index_cell = allocator.alloc_common_range_cell();
        let external_host_call_index_cell = allocator.alloc_common_range_cell();
        let sp_cell = allocator.alloc_common_range_cell();
        let mpages_cell = allocator.alloc_common_range_cell();
        let frame_id_cell = allocator.alloc_u32_state_cell();
        let eid_cell = allocator.alloc_u32_state_cell();
        let fid_cell = allocator.alloc_common_range_cell();
        let iid_cell = allocator.alloc_common_range_cell();
        let maximal_memory_pages_cell = allocator.alloc_common_range_cell();

        // We only need to enable equality for the cells of states
        let used_common_range_cells_for_state = allocator
            .free_cells
            .get(&EventTableCellType::CommonRange)
            .unwrap();
        allocator.enable_equality(
            meta,
            &EventTableCellType::CommonRange,
            used_common_range_cells_for_state.0
                + (used_common_range_cells_for_state.1 != 0) as usize,
        );

        let used_unlimited_cells_for_state = allocator
            .free_cells
            .get(&EventTableCellType::Unlimited)
            .unwrap();
        allocator.enable_equality(
            meta,
            &EventTableCellType::Unlimited,
            used_unlimited_cells_for_state.0 + (used_unlimited_cells_for_state.1 != 0) as usize,
        );

        let itable_lookup_cell = allocator.alloc_unlimited_cell();
        let brtable_lookup_cell = allocator.alloc_unlimited_cell();
        let jtable_lookup_cell = allocator.alloc_unlimited_cell();
        let is_returned_cell = allocator.alloc_bit_cell();
        let pow_table_lookup_modulus_cell = allocator.alloc_unlimited_cell();
        let pow_table_lookup_power_cell = allocator.alloc_unlimited_cell();
        let external_foreign_call_lookup_cell = allocator.alloc_unlimited_cell();
        let bit_table_lookup_cells = allocator.alloc_bit_table_lookup_cells();

        let mut foreign_table_reserved_lookup_cells = [(); FOREIGN_LOOKUP_CAPABILITY]
            .map(|_| allocator.alloc_unlimited_cell())
            .into_iter();

        let common_config = EventTableCommonConfig {
            enabled_cell,
            ops,
            rest_mops_cell,
            rest_call_ops_cell,
            rest_return_ops_cell,
            input_index_cell,
            context_input_index_cell,
            context_output_index_cell,
            external_host_call_index_cell,
            sp_cell,
            mpages_cell,
            frame_id_cell,
            eid_cell,
            fid_cell,
            iid_cell,
            maximal_memory_pages_cell,
            itable_lookup_cell,
            brtable_lookup_cell,
            jtable_lookup_cell,
            is_returned_cell,
            pow_table_lookup_modulus_cell,
            pow_table_lookup_power_cell,
            bit_table_lookup_cells,
            external_foreign_call_lookup_cell,
        };

        let mut op_bitmaps: BTreeMap<OpcodeClassPlain, usize> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClassPlain, OpcodeConfig<F>> = BTreeMap::new();

        let mut profiler = AllocatorFreeCellsProfiler::new(&allocator);

        macro_rules! configure {
            ($op:expr, $x:ident) => {
                let op = OpcodeClassPlain($op as usize);

                let foreign_table_configs = BTreeMap::new();
                let mut constraint_builder = ConstraintBuilder::new(meta, &foreign_table_configs);

                let mut allocator = allocator.clone();
                let config = $x::configure(&common_config, &mut allocator, &mut constraint_builder);

                constraint_builder.finalize(|meta| {
                    (fixed_curr!(meta, step_sel), ops[op.index()].curr_expr(meta))
                });

                op_bitmaps.insert(op, op.index());
                op_configs.insert(op, OpcodeConfig::<F>(config));

                profiler.update(&allocator);
            };
        }

        configure!(OpcodeClass::BinShift, BinShiftConfigBuilder);
        configure!(OpcodeClass::Bin, BinConfigBuilder);
        configure!(OpcodeClass::BrIfEqz, BrIfEqzConfigBuilder);
        configure!(OpcodeClass::BrIf, BrIfConfigBuilder);
        configure!(OpcodeClass::Br, BrConfigBuilder);
        configure!(OpcodeClass::Call, CallConfigBuilder);
        configure!(OpcodeClass::CallHost, ExternalCallHostCircuitConfigBuilder);
        configure!(OpcodeClass::Const, ConstConfigBuilder);
        configure!(OpcodeClass::Conversion, ConversionConfigBuilder);
        configure!(OpcodeClass::Drop, DropConfigBuilder);
        configure!(OpcodeClass::GlobalGet, GlobalGetConfigBuilder);
        configure!(OpcodeClass::GlobalSet, GlobalSetConfigBuilder);
        configure!(OpcodeClass::LocalGet, LocalGetConfigBuilder);
        configure!(OpcodeClass::LocalSet, LocalSetConfigBuilder);
        configure!(OpcodeClass::LocalTee, LocalTeeConfigBuilder);
        configure!(OpcodeClass::Rel, RelConfigBuilder);
        configure!(OpcodeClass::Return, ReturnConfigBuilder);
        configure!(OpcodeClass::Select, SelectConfigBuilder);
        configure!(OpcodeClass::Test, TestConfigBuilder);
        configure!(OpcodeClass::Unary, UnaryConfigBuilder);
        configure!(OpcodeClass::Load, LoadConfigBuilder);
        configure!(OpcodeClass::Store, StoreConfigBuilder);
        configure!(OpcodeClass::BinBit, BinBitConfigBuilder);
        configure!(OpcodeClass::MemorySize, MemorySizeConfigBuilder);
        configure!(OpcodeClass::MemoryGrow, MemoryGrowConfigBuilder);
        configure!(OpcodeClass::BrTable, BrTableConfigBuilder);
        configure!(OpcodeClass::CallIndirect, CallIndirectConfigBuilder);

        macro_rules! configure_foreign {
            ($x:ident, $i:expr) => {
                let builder = $x::new($i);
                let op = OpcodeClass::ForeignPluginStart as usize + $i;
                let op = OpcodeClassPlain(op);

                let mut constraint_builder = ConstraintBuilder::new(meta, foreign_table_configs);
                let mut allocator = allocator.clone();

                let config = builder.configure(
                    &common_config,
                    &mut allocator,
                    &mut constraint_builder,
                    &mut foreign_table_reserved_lookup_cells,
                );

                constraint_builder.finalize(|meta| {
                    (fixed_curr!(meta, step_sel), ops[op.index()].curr_expr(meta))
                });

                op_bitmaps.insert(op, op.index());
                op_configs.insert(op, OpcodeConfig(config));

                profiler.update(&allocator);
            };
        }
        configure_foreign!(ETableWasmInputHelperTableConfigBuilder, 0);
        configure_foreign!(ETableContextHelperTableConfigBuilder, 1);
        configure_foreign!(ETableRequireHelperTableConfigBuilder, 2);

        profiler.assert_no_free_cells(&allocator);

        meta.create_gate("c1. enable seq", |meta| {
            vec![
                enabled_cell.next_expr(meta)
                    * (enabled_cell.curr_expr(meta) - constant_from!(1))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        meta.create_gate("c4. opcode_bit lvl sum equals to 1", |meta| {
            vec![
                ops.map(|x| x.curr_expr(meta))
                    .into_iter()
                    .reduce(|acc, x| acc + x)
                    .unwrap()
                    - enabled_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|expr| expr * fixed_curr!(meta, step_sel))
            .collect::<Vec<_>>()
        });

        /*
         * How `* enabled_cell.curr_expr(meta)` effects on the separate step:
         *    1. constrains the relation between the last step and termination.
         *    2. ignores rows following the termination step.
         */
        let sum_ops_expr_with_init = |init: Expression<F>,
                                      meta: &mut VirtualCells<'_, F>,
                                      get_expr: &dyn Fn(
            &mut VirtualCells<'_, F>,
            &OpcodeConfig<F>,
        ) -> Option<Expression<F>>| {
            op_bitmaps
                .iter()
                .filter_map(|(op, op_index)| {
                    get_expr(meta, op_configs.get(op).unwrap())
                        .map(|expr| expr * ops[*op_index].curr_expr(meta))
                })
                .fold(init, |acc, x| acc + x)
                * fixed_curr!(meta, step_sel)
        };

        let sum_ops_expr = |meta: &mut VirtualCells<'_, F>,
                            get_expr: &dyn Fn(
            &mut VirtualCells<'_, F>,
            &OpcodeConfig<F>,
        ) -> Option<Expression<F>>| {
            op_bitmaps
                .iter()
                .filter_map(|(op, op_index)| {
                    get_expr(meta, op_configs.get(op).unwrap())
                        .map(|expr| expr * ops[*op_index].curr_expr(meta))
                })
                .reduce(|acc, x| acc + x)
                .unwrap()
        };

        meta.create_gate("c5a. rest_mops change", |meta| {
            vec![sum_ops_expr_with_init(
                rest_mops_cell.next_expr(meta) - rest_mops_cell.curr_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| config.0.mops(meta),
            )]
        });

        meta.create_gate("c5b. rest jops change", |meta| {
            vec![
                sum_ops_expr_with_init(
                    rest_call_ops_cell.next_expr(meta) - rest_call_ops_cell.curr_expr(meta),
                    meta,
                    &|meta, config: &OpcodeConfig<F>| config.0.call_ops_expr(meta),
                ),
                sum_ops_expr_with_init(
                    rest_return_ops_cell.next_expr(meta) - rest_return_ops_cell.curr_expr(meta),
                    meta,
                    &|meta, config: &OpcodeConfig<F>| config.0.return_ops_expr(meta),
                ),
            ]
        });

        meta.create_gate("c5c. input_index change", |meta| {
            vec![sum_ops_expr_with_init(
                input_index_cell.curr_expr(meta) - input_index_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config.0.input_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c5d. external_host_call_index change", |meta| {
            vec![sum_ops_expr_with_init(
                external_host_call_index_cell.curr_expr(meta)
                    - external_host_call_index_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config
                        .0
                        .external_host_call_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c5e. sp change", |meta| {
            vec![sum_ops_expr_with_init(
                sp_cell.curr_expr(meta) - sp_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| config.0.sp_diff(meta),
            )]
        });

        meta.create_gate("c5f. mpages change", |meta| {
            vec![sum_ops_expr_with_init(
                mpages_cell.curr_expr(meta) - mpages_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| config.0.allocated_memory_pages_diff(meta),
            )]
        });

        meta.create_gate("c5g. context_input_index change", |meta| {
            vec![sum_ops_expr_with_init(
                context_input_index_cell.curr_expr(meta) - context_input_index_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config.0.context_input_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c5h. context_output_index change", |meta| {
            vec![sum_ops_expr_with_init(
                context_output_index_cell.curr_expr(meta)
                    - context_output_index_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config.0.context_output_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c6a. eid change", |meta| {
            vec![
                (eid_cell.next_expr(meta)
                    - eid_cell.curr_expr(meta)
                    - enabled_cell.curr_expr(meta))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        meta.create_gate("c6b. fid change", |meta| {
            vec![sum_ops_expr_with_init(
                fid_cell.curr_expr(meta) - fid_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config
                        .0
                        .next_fid(meta, &common_config)
                        .map(|x| x - fid_cell.curr_expr(meta))
                },
            )]
        });

        meta.create_gate("c6c. iid change", |meta| {
            vec![sum_ops_expr_with_init(
                iid_cell.next_expr(meta) - iid_cell.curr_expr(meta) - enabled_cell.curr_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config
                        .0
                        .next_iid(meta, &common_config)
                        .map(|x| iid_cell.curr_expr(meta) + enabled_cell.curr_expr(meta) - x)
                },
            )]
        });

        meta.create_gate("c6d. frame_id change", |meta| {
            vec![sum_ops_expr_with_init(
                frame_id_cell.curr_expr(meta) - frame_id_cell.next_expr(meta),
                meta,
                &|meta, config: &OpcodeConfig<F>| {
                    config
                        .0
                        .next_frame_id(meta, &common_config)
                        .map(|x| x - frame_id_cell.curr_expr(meta))
                },
            )]
        });

        meta.create_gate("c7. itable_lookup_encode", |meta| {
            let opcode = sum_ops_expr(meta, &|meta, config: &OpcodeConfig<F>| {
                Some(config.0.opcode(meta))
            });
            vec![
                (encode_instruction_table_entry(fid_cell.expr(meta), iid_cell.expr(meta), opcode)
                    - itable_lookup_cell.curr_expr(meta))
                    * enabled_cell.curr_expr(meta)
                    * fixed_curr!(meta, step_sel),
            ]
        });

        image_table.instruction_lookup(meta, "c8a. itable_lookup in itable", |meta| {
            itable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        image_table.br_table_lookup(meta, "c8b. brtable_lookup in brtable", |meta| {
            brtable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        jtable.configure_lookup_in_frame_table(meta, "c8c. jtable_lookup in jtable", |meta| {
            (
                fixed_curr!(meta, step_sel),
                common_config.is_returned_cell.curr_expr(meta) * fixed_curr!(meta, step_sel),
                common_config.jtable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel),
            )
        });

        rtable.configure_in_pow_set(
            meta,
            "c8d. pow_table_lookup in pow_table",
            |meta| pow_table_lookup_power_cell.curr_expr(meta),
            |meta| pow_table_lookup_modulus_cell.curr_expr(meta),
            |meta| fixed_curr!(meta, step_sel),
        );

        external_host_call_table.configure_in_table(
            meta,
            "c8g. external_foreign_call_lookup in foreign table",
            |meta| {
                vec![
                    external_foreign_call_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel),
                ]
            },
        );

        bit_table.configure_in_table(meta, "c8f: bit_table_lookup in bit_table", |meta| {
            (
                fixed_curr!(meta, step_sel),
                fixed_curr!(meta, step_sel) * bit_table_lookup_cells.op.expr(meta),
                fixed_curr!(meta, step_sel) * bit_table_lookup_cells.left.expr(meta),
                fixed_curr!(meta, step_sel) * bit_table_lookup_cells.right.expr(meta),
                fixed_curr!(meta, step_sel) * bit_table_lookup_cells.result.expr(meta),
            )
        });

        meta.create_gate("c9. maximal memory pages consistent", |meta| {
            vec![
                (maximal_memory_pages_cell.next_expr(meta)
                    - maximal_memory_pages_cell.curr_expr(meta))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        Self {
            step_sel,
            common_config,
            op_configs: Arc::new(op_configs),
        }
    }
}

#[derive(Clone)]
pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig<F>,
    // The maximal number of entries(which sel = 1) of etable
    capability: usize,
}

impl<F: FieldExt> EventTableChip<F> {
    pub(super) fn new(
        config: EventTableConfig<F>,
        capability: usize,
        max_available_rows: usize,
    ) -> Self {
        assert!(capability * EVENT_TABLE_ENTRY_ROWS as usize <= max_available_rows);

        Self { config, capability }
    }
}
