use self::allocator::*;
use super::{
    brtable::BrTableConfig, cell::*, config::max_etable_rows, itable::InstructionTableConfig,
    jtable::JumpTableConfig, mtable_v2::MemoryTableConfig, rtable::RangeTableConfig,
    traits::ConfigureLookupTable, utils::Context, CircuitConfigure, Lookup,
};
use crate::{constant_from, fixed_curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};
use specs::{
    configure_table::ConfigureTable, encode::instruction_table::encode_instruction_table_entry,
    etable::EventTableEntry, itable::OpcodeClassPlain,
};
use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

mod allocator;
mod assign;
mod op_configure;

pub(crate) const ESTEP_SIZE: i32 = 4;
pub(crate) const OP_LVL1_BITS: usize = 6;
pub(crate) const OP_LVL2_BITS: usize = 6;

#[derive(Clone)]
pub struct Status {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub last_jump_eid: u32,
    pub allocated_memory_pages: u32,
}

pub struct StepStatus<'a> {
    pub current: &'a Status,
    pub next: &'a Status,
    pub current_external_host_call_index: usize,
    pub configure: ConfigureTable,
}

#[derive(Clone)]
pub struct EventTableCommonConfig<F: FieldExt> {
    enabled_cell: AllocatedBitCell<F>,

    lvl1_bits: [AllocatedBitCell<F>; 6],
    lvl2_bits: [AllocatedBitCell<F>; 6],

    rest_mops_cell: AllocatedCommonRangeCell<F>,
    rest_jops_cell: AllocatedCommonRangeCell<F>,
    input_index_cell: AllocatedCommonRangeCell<F>,
    external_host_call_index_cell: AllocatedCommonRangeCell<F>,
    sp_cell: AllocatedCommonRangeCell<F>,
    mpages_cell: AllocatedCommonRangeCell<F>,
    frame_id_cell: AllocatedCommonRangeCell<F>,
    eid_cell: AllocatedCommonRangeCell<F>,
    fid_cell: AllocatedCommonRangeCell<F>,
    iid_cell: AllocatedCommonRangeCell<F>,

    itable_lookup_cell: AllocatedUnlimitedCell<F>,
    brtable_lookup_cell: AllocatedUnlimitedCell<F>,
    jtable_lookup_cell: AllocatedUnlimitedCell<F>,
    pow_table_lookup_cell: AllocatedUnlimitedCell<F>,
    olb_table_lookup_cell: AllocatedUnlimitedCell<F>,
}

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub(self) fn allocate_opcode_bit_cell(
        &self,
        opcode_class_plain: OpcodeClassPlain,
    ) -> (AllocatedBitCell<F>, AllocatedBitCell<F>) {
        // OpcodeClassPlain starts from 1.
        let idx = opcode_class_plain.0 - 1;

        assert!(idx < OP_LVL1_BITS * OP_LVL2_BITS);

        (
            *self.lvl1_bits.get(idx / OP_LVL2_BITS).unwrap(),
            *self.lvl2_bits.get(idx % OP_LVL2_BITS).unwrap(),
        )
    }
}

pub(in crate::circuits::etable_v2) struct ConstraintBuilder<'a, F: FieldExt> {
    meta: &'a mut ConstraintSystem<F>,
    constraints: Vec<(
        &'static str,
        Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    )>,
    lookups: BTreeMap<
        &'static str,
        Vec<(
            &'static str,
            Box<dyn Fn(&mut VirtualCells<F>) -> Expression<F>>,
        )>,
    >,
}

pub(in crate::circuits::etable_v2) trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        common: &mut EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error>;
    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        _entry: &EventTableEntry,
    ) -> u64 {
        0u64
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
    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
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

#[derive(Clone)]
pub struct EventTableConfig<F: FieldExt> {
    pub sel: Column<Fixed>,
    pub step_sel: Column<Fixed>,
    pub common_config: EventTableCommonConfig<F>,
    op_bitmaps: BTreeMap<OpcodeClassPlain, (usize, usize)>,
    op_configs: BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
}

impl<F: FieldExt> EventTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        _circuit_configure: &CircuitConfigure,
        rtable: &RangeTableConfig<F>,
        itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
        brtable: &BrTableConfig<F>,
        _opcode_set: &HashSet<OpcodeClassPlain>,
    ) -> EventTableConfig<F> {
        let sel = meta.fixed_column();
        let step_sel = meta.fixed_column();

        let mut allocator = EventTableCellAllocator::new(meta, rtable, mtable, cols);
        allocator.enable_equality(meta, &EventTableCellType::CommonRange);

        let lvl1_bits = [0; OP_LVL1_BITS].map(|_| allocator.alloc_bit_cell());
        let lvl2_bits = [0; OP_LVL2_BITS].map(|_| allocator.alloc_bit_cell());

        let enabled_cell = allocator.alloc_bit_cell();
        let rest_mops_cell = allocator.alloc_common_range_cell();
        let rest_jops_cell = allocator.alloc_common_range_cell();
        let input_index_cell = allocator.alloc_common_range_cell();
        let external_host_call_index_cell = allocator.alloc_common_range_cell();
        let sp_cell = allocator.alloc_common_range_cell();
        let mpages_cell = allocator.alloc_common_range_cell();
        let frame_id_cell = allocator.alloc_common_range_cell();
        let eid_cell = allocator.alloc_common_range_cell();
        let fid_cell = allocator.alloc_common_range_cell();
        let iid_cell = allocator.alloc_common_range_cell();

        let itable_lookup_cell = allocator.alloc_unlimited_cell();
        let brtable_lookup_cell = allocator.alloc_unlimited_cell();
        let jtable_lookup_cell = allocator.alloc_unlimited_cell();
        let pow_table_lookup_cell = allocator.alloc_unlimited_cell();
        let olb_table_lookup_cell = allocator.alloc_unlimited_cell();

        let common_config = EventTableCommonConfig {
            enabled_cell,
            lvl1_bits,
            lvl2_bits,
            rest_mops_cell,
            rest_jops_cell,
            input_index_cell,
            external_host_call_index_cell,
            sp_cell,
            mpages_cell,
            frame_id_cell,
            eid_cell,
            fid_cell,
            iid_cell,
            itable_lookup_cell,
            brtable_lookup_cell,
            jtable_lookup_cell,
            pow_table_lookup_cell,
            olb_table_lookup_cell,
        };

        let mut op_bitmaps: BTreeMap<OpcodeClassPlain, (usize, usize)> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

        {}

        meta.create_gate("c1. enable seq", |meta| {
            vec![
                enabled_cell.next_expr(meta)
                    * (enabled_cell.curr_expr(meta) - constant_from!(1))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        meta.create_gate("c4. opcode_bit lvl sum equals to 1", |meta| {
            vec![
                lvl1_bits
                    .map(|x| x.curr_expr(meta))
                    .into_iter()
                    .reduce(|acc, x| acc + x)
                    .unwrap()
                    - constant_from!(1),
                lvl2_bits
                    .map(|x| x.curr_expr(meta))
                    .into_iter()
                    .reduce(|acc, x| acc + x)
                    .unwrap()
                    - constant_from!(1),
            ]
        });

        let sum_ops_expr_with_init = |init: Expression<F>,
                                      meta: &mut VirtualCells<'_, F>,
                                      get_expr: &dyn Fn(
            &mut VirtualCells<'_, F>,
            &Rc<Box<dyn EventTableOpcodeConfig<F>>>,
        ) -> Option<Expression<F>>| {
            op_bitmaps
                .iter()
                .filter_map(|(op, (lvl1, lvl2))| {
                    get_expr(meta, op_configs.get(op).unwrap()).map(|expr| {
                        expr * fixed_curr!(meta, step_sel)
                            * lvl1_bits[*lvl1].curr_expr(meta)
                            * lvl2_bits[*lvl2].curr_expr(meta)
                    })
                })
                .fold(init, |acc, x| acc + x)
        };

        let sum_ops_expr = |meta: &mut VirtualCells<'_, F>,
                            get_expr: &dyn Fn(
            &mut VirtualCells<'_, F>,
            &Rc<Box<dyn EventTableOpcodeConfig<F>>>,
        ) -> Option<Expression<F>>| {
            op_bitmaps
                .iter()
                .filter_map(|(op, (lvl1, lvl2))| {
                    get_expr(meta, op_configs.get(op).unwrap()).map(|expr| {
                        expr * fixed_curr!(meta, step_sel)
                            * lvl1_bits[*lvl1].curr_expr(meta)
                            * lvl2_bits[*lvl2].curr_expr(meta)
                    })
                })
                .reduce(|acc, x| acc + x)
                .unwrap()
        };

        meta.create_gate("c5a. rest_mops change", |meta| {
            vec![sum_ops_expr_with_init(
                rest_mops_cell.next_expr(meta) - rest_mops_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| config.mops(meta),
            )]
        });

        meta.create_gate("c5b. rest_jops change", |meta| {
            vec![sum_ops_expr_with_init(
                rest_jops_cell.next_expr(meta) - rest_jops_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| config.jops(meta),
            )]
        });

        meta.create_gate("c5c. input_index change", |meta| {
            vec![sum_ops_expr_with_init(
                input_index_cell.next_expr(meta) - input_index_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config.input_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c5d. external_host_call_index change", |meta| {
            vec![sum_ops_expr_with_init(
                external_host_call_index_cell.next_expr(meta)
                    - external_host_call_index_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config.external_host_call_index_increase(meta, &common_config)
                },
            )]
        });

        meta.create_gate("c5e. sp change", |meta| {
            vec![sum_ops_expr_with_init(
                sp_cell.next_expr(meta) - sp_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| config.sp_diff(meta),
            )]
        });

        meta.create_gate("c5f. mpages change", |meta| {
            vec![sum_ops_expr_with_init(
                mpages_cell.next_expr(meta) - mpages_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config.allocated_memory_pages_diff(meta)
                },
            )]
        });

        meta.create_gate("c6a. eid change", |meta| {
            vec![eid_cell.next_expr(meta) - eid_cell.curr_expr(meta) - constant_from!(1)]
        });

        meta.create_gate("c6b. fid change", |meta| {
            vec![sum_ops_expr_with_init(
                fid_cell.next_expr(meta) - fid_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config
                        .next_fid(meta, &common_config)
                        .map(|x| x - fid_cell.curr_expr(meta))
                },
            )]
        });

        meta.create_gate("c6c. iid change", |meta| {
            vec![sum_ops_expr_with_init(
                iid_cell.next_expr(meta) - iid_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config
                        .next_iid(meta, &common_config)
                        .map(|x| x - iid_cell.curr_expr(meta))
                },
            )]
        });

        meta.create_gate("c6d. frame_id change", |meta| {
            vec![sum_ops_expr_with_init(
                frame_id_cell.next_expr(meta) - frame_id_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| {
                    config
                        .next_frame_id(meta, &common_config)
                        .map(|x| x - frame_id_cell.curr_expr(meta))
                },
            )]
        });

        meta.create_gate("c7. itable_lookup_encode", |meta| {
            let opcode = sum_ops_expr(
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| Some(config.opcode(meta)),
            );
            vec![
                encode_instruction_table_entry(fid_cell.expr(meta), iid_cell.expr(meta), opcode)
                    * fixed_curr!(meta, step_sel),
            ]
        });

        jtable.configure_in_table(meta, "c8a. itable_lookup in itable", |meta| {
            jtable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        itable.configure_in_table(meta, "c8b. brtable_lookup in brtable", |meta| {
            itable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        brtable.configure_in_table(meta, "c8c. jtable_lookup in jtable", |meta| {
            brtable_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        rtable.configure_in_pow_set(meta, "c8d. pow_table_lookup in pow_table", |meta| {
            pow_table_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel)
        });

        rtable.configure_in_offset_len_bits_set(
            meta,
            "c8e. olb_table_lookup in offset_len_bits_table",
            |meta| olb_table_lookup_cell.curr_expr(meta) * fixed_curr!(meta, step_sel),
        );

        Self {
            sel,
            step_sel,
            common_config,
            op_bitmaps,
            op_configs,
        }
    }
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig<F>,
    max_available_rows: usize,
}

impl<F: FieldExt> EventTableChip<F> {
    pub(super) fn new(config: EventTableConfig<F>) -> Self {
        Self {
            config,
            max_available_rows: max_etable_rows() as usize / ESTEP_SIZE as usize
                * ESTEP_SIZE as usize,
        }
    }
}
