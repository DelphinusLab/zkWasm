use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
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
use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::rc::Rc;

use super::*;

pub mod expression;
pub mod op_configure;

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
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

const U4_COLUMNS: usize = 4usize;

pub(crate) enum EventTableBitColumnRotation {
    Enable = 0,
    Max,
}

pub(crate) enum EventTableCommonRangeColumnRotation {
    RestMOps = 0,
    RestJOps,
    EID,
    MOID,
    FID,
    IID,
    MMID,
    SP,
    Max,
}

pub(crate) enum EventTableUnlimitColumnRotation {
    ITableLookup = 0,
    JTableLookup,
    MTableLookupStart,
    U64Start = 8,
    SharedStart = 12,
}

#[derive(Clone)]
pub struct EventTableCommonConfig {
    pub sel: Column<Fixed>,
    pub block_first_line_sel: Column<Fixed>,

    // Rotation:
    // 0 enable
    pub shared_bits: Column<Advice>,
    pub opcode_bits: Column<Advice>,

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

    pub itable_lookup: Column<Fixed>,
    pub jtable_lookup: Column<Fixed>,
    pub mtable_lookup: Column<Fixed>,
    // Rotation
    // 0      itable lookup
    // 1      jtable lookup
    // 2..7   mtable lookup
    // 8..11  u4 sum
    // 12..15 shared
    pub aux: Column<Advice>,

    pub u4_shared: [Column<Advice>; U4_COLUMNS],
}

#[derive(Clone)]
pub struct EventTableConfig<F: FieldExt> {
    common_config: EventTableCommonConfig,
    op_bitmaps: BTreeMap<OpcodeClass, (i32, i32)>,
    op_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
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
        opcode_set: &BTreeSet<OpcodeClass>,
    ) -> Self {
        let sel = meta.fixed_column();
        let block_first_line_sel = meta.fixed_column();
        let shared_bits = cols.next().unwrap();
        let opcode_bits = cols.next().unwrap();

        let aux_in_common = cols.next().unwrap();
        let aux = cols.next().unwrap();

        let itable_lookup = meta.fixed_column();
        let jtable_lookup = meta.fixed_column();
        let mtable_lookup = meta.fixed_column();

        let u4_shared = [0; 4].map(|_| cols.next().unwrap());

        meta.enable_equality(aux_in_common);

        meta.create_gate("etable bits", |meta| {
            vec![
                curr!(meta, shared_bits) * (curr!(meta, shared_bits) - constant_from!(1)),
                curr!(meta, opcode_bits) * (curr!(meta, opcode_bits) - constant_from!(1)),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, sel))
            .collect::<Vec<_>>()
        });

        rtable.configure_in_common_range(meta, "etable aux in common", |meta| {
            curr!(meta, aux_in_common) * fixed_curr!(meta, sel)
        });

        for i in 0..U4_COLUMNS {
            rtable.configure_in_u4_range(meta, "etable u4", |meta| {
                curr!(meta, u4_shared[i]) * fixed_curr!(meta, sel)
            });
        }

        itable.configure_in_table(meta, "etable itable lookup", |meta| {
            curr!(meta, aux) * fixed_curr!(meta, itable_lookup)
        });

        for i in 0..U4_COLUMNS {
            meta.create_gate("etable u64", |meta| {
                let mut acc = nextn!(
                    meta,
                    aux,
                    EventTableUnlimitColumnRotation::U64Start as i32 + i as i32
                );
                let mut base = 1u64;
                for j in 0..16 {
                    acc = acc - nextn!(meta, u4_shared[i], j) * constant_from!(base);
                    base <<= 8;
                }

                vec![acc * fixed_curr!(meta, block_first_line_sel)]
            });
        }

        //TODO: also add lookups for mtable & jtable

        let common_config = EventTableCommonConfig {
            sel,
            block_first_line_sel,
            shared_bits,
            opcode_bits,
            aux_in_common,
            itable_lookup,
            jtable_lookup,
            mtable_lookup,
            aux,
            u4_shared,
        };

        const MAX_OP_LVL1: i32 = 8;
        const MAX_OP_LVL2: i32 = ETABLE_STEP_SIZE as i32;

        let mut op_lvl1 = 0;
        let mut op_lvl2 = MAX_OP_LVL1;

        let mut op_bitmaps_vec: Vec<(i32, i32)> = vec![];
        let mut op_bitmaps: BTreeMap<OpcodeClass, (i32, i32)> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

        macro_rules! configure [
            ($($x:ident),*) => ({
                let curr_op_lvl2 = op_lvl2;
                op_lvl2 += 1;
                if op_lvl2 == MAX_OP_LVL2 {
                    op_lvl2 = 1;
                    op_lvl1 += 1;
                    assert!(op_lvl1 < MAX_OP_LVL1);
                }

                let mut opcode_bitmaps_iter = opcode_bitmaps_vec.iter();
                $(
                    let opcode_bit = opcode_bitmaps_iter.next().unwrap();
                    let config = $x::configure(
                        meta,
                        &common_config,
                        |meta| fixed_curr!(meta, common_config.block_first_line_sel)
                    );
                    opcode_bitmaps.insert(config.opcode_class(), (op_lvl1, op_lvl2));
                    opcode_configs.insert(config.opcode_class(), Rc::new(config));
                )*
            })
        ];

        Self {
            common_config,
            op_bitmaps,
            op_configs,
            _mark: PhantomData,
        }
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
