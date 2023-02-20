use std::{
    collections::{BTreeMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};
use specs::{etable::EventTableEntry, itable::OpcodeClassPlain};

use crate::{constant_from, fixed_curr};

use self::allocator::{CellAllocator, CellExpression, ETableCellType};

use super::{
    etable_compact::{EventTableCommonConfig, StepStatus},
    rtable::RangeTableConfig,
    utils::Context,
    CircuitConfigure,
};

mod allocator;

pub(crate) const ESTEP_SIZE: i32 = 4;
pub(crate) const OP_LVL1_BITS: usize = 5;
pub(crate) const OP_LVL2_BITS: usize = 5;

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
    fn is_host_public_input(&self, _step: &StepStatus, _entry: &EventTableEntry) -> bool {
        false
    }
    fn external_host_call_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> bool {
        false
    }
}

pub struct ETableConfig<F: FieldExt> {
    pub sel: Column<Fixed>,
    pub step_sel: Column<Fixed>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> ETableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        _circuit_configure: &CircuitConfigure,
        rtable: &RangeTableConfig<F>,
        _opcode_set: &HashSet<OpcodeClassPlain>,
    ) -> ETableConfig<F> {
        let sel = meta.fixed_column();
        let step_sel = meta.fixed_column();

        let mut allocator = CellAllocator::new(meta, rtable, cols);
        allocator.enable_equality(meta, &ETableCellType::CommonRange);

        let enabled_cell = allocator.alloc_bit_cell();
        let lvl1_bits = [0; OP_LVL1_BITS].map(|_| allocator.alloc_bit_cell());
        let lvl2_bits = [0; OP_LVL2_BITS].map(|_| allocator.alloc_bit_cell());

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
        let aux_table_lookup_cell = allocator.alloc_unlimited_cell();

        let mut op_bitmaps: BTreeMap<OpcodeClassPlain, (usize, usize)> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

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

        let fold_ops_expr = |init: Expression<F>,
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

        meta.create_gate("c5a. rest_mops change", |meta| {
            vec![fold_ops_expr(
                rest_mops_cell.next_expr(meta) - rest_mops_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| config.mops(meta),
            )]
        });

        meta.create_gate("c5b. rest_jops change", |meta| {
            vec![fold_ops_expr(
                rest_jops_cell.next_expr(meta) - rest_jops_cell.curr_expr(meta),
                meta,
                &|meta, config: &Rc<Box<dyn EventTableOpcodeConfig<F>>>| config.jops(meta),
            )]
        });

        Self {
            sel,
            step_sel,
            _mark: PhantomData,
        }
    }
}
