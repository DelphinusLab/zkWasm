use crate::circuits::utils::bn_to_field;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;
use strum::IntoEnumIterator;

use super::*;

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error>;
    fn opcode_class(&self) -> OpcodeClass;
    /// For br and return
    fn extra_mops(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from!(0)
    }
    fn handle_iid(&self) -> bool {
        false
    }
    fn handle_moid(&self) -> bool {
        false
    }
    fn handle_fid(&self) -> bool {
        false
    }
    fn last_jump_eid_change(&self) -> bool {
        false
    }
}

const ETABLE_ROWS: usize = 1usize << 16;
const ETABLE_STEP_SIZE: usize = 16usize;

pub const ROTATION_ENABLE: i32 = 0;

pub const ROTATION_REST_MOPS: i32 = 0;
pub const ROTATION_REST_JOPS: i32 = 1;
pub const ROTATION_EID: i32 = 2;
pub const ROTATION_MOID: i32 = 3;
pub const ROTATION_FID: i32 = 4;
pub const ROTATION_IID: i32 = 5;
pub const ROTATION_MMID: i32 = 6;
pub const ROTATION_SP: i32 = 7;
pub const ROTATION_LAST_JUMP_EID: i32 = 8;

#[derive(Clone)]
pub struct EventTableCommonConfig {
    pub sel: Column<Fixed>,
    pub sel_block_first_line: Column<Fixed>,

    // Rotation:
    // 0 enable
    pub bit_g1: Column<Advice>,
    pub bit_g2: Column<Advice>,

    // Rotation:
    // 0 rest_mops
    // 1 rest_jops
    // 2 eid
    // 3 moid
    // 4 fid
    // 5 iid
    // 6 mmid
    // 7 sp
    pub aux_in_common: Column<Advice>,

    pub itable_lookups: Column<Advice>,
    pub mtable_lookups: Column<Advice>,
    pub jtable_lookups: Column<Advice>,

    pub u4_g1: Column<Advice>,
    pub u4_g2: Column<Advice>,
    pub u4_g3: Column<Advice>,
    pub u4_g4: Column<Advice>,
}

#[derive(Clone)]
pub struct EventTableConfig<F: FieldExt> {
    common_config: EventTableCommonConfig,
    opcode_bitmaps: BTreeMap<OpcodeClass, Column<Advice>>,
    opcode_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
    ) -> Self {
        todo!();
    }
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> EventTableChip<F> {
    pub fn new(config: EventTableConfig<F>) -> Self {
        EventTableChip {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        entries: &Vec<EventTableEntry>,
    ) -> Result<(Cell, Cell), Error> {
        todo!();
    }
}
