use self::op_configure::EventTableOpcodeConfig;
use super::*;
use crate::circuits::etable_compact::op_configure::op_const::ConstConfigBuilder;
use crate::circuits::etable_compact::op_configure::op_drop::DropConfigBuilder;
use crate::circuits::etable_compact::op_configure::op_return::ReturnConfigBuilder;
use crate::circuits::etable_compact::op_configure::EventTableCellAllocator;
use crate::circuits::etable_compact::op_configure::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::bn_to_field;
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
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::rc::Rc;

pub mod expression;
pub mod op_configure;

// TODO:
// 1. add constraints for termination
// 2. add input output for circuits

const ETABLE_ROWS: usize = 1usize << 16;
const ETABLE_STEP_SIZE: usize = 16usize;
const U4_COLUMNS: usize = 4usize;
const MTABLE_LOOKUPS_SIZE: usize = 4usize;
const MAX_OP_LVL1: i32 = 8;
const MAX_OP_LVL2: i32 = ETABLE_STEP_SIZE as i32;

fn opclass_to_two_level(class: OpcodeClass) -> (usize, usize) {
    let mut id = class as i32;
    assert!(id <= MAX_OP_LVL1 * (MAX_OP_LVL2 - MAX_OP_LVL1));

    id -= 1;

    (
        (id / MAX_OP_LVL1) as usize,
        ((id % MAX_OP_LVL1) + 8) as usize,
    )
}

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
    LastJumpEid,
    Max,
}

pub(crate) enum EventTableUnlimitColumnRotation {
    ITableLookup = 0,
    JTableLookup,
    MTableLookupStart,
    U64Start = 6,
    SharedStart = 10,
}

pub(self) enum MLookupItem {
    First = 0,
    Second,
    Third,
    Fourth,
}

impl From<usize> for MLookupItem {
    fn from(i: usize) -> Self {
        match i {
            0 => Self::First,
            1 => Self::Second,
            2 => Self::Third,
            3 => Self::Fourth,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
struct Status {
    eid: u64,
    moid: u16,
    fid: u16,
    iid: u16,
    mmid: u16,
    sp: u64,
    last_jump_eid: u64,
}

pub(self) struct StepStatus<'a> {
    current: &'a Status,
    next: &'a Status,
}

impl TryFrom<u32> for MLookupItem {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::First),
            1 => Ok(Self::Second),
            2 => Ok(Self::Third),
            3 => Ok(Self::Fourth),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub(super) struct EventTableCommonConfig<F> {
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
    // 8 last_jump_eid
    pub state: Column<Advice>,

    pub itable_lookup: Column<Fixed>,
    pub jtable_lookup: Column<Fixed>,
    pub mtable_lookup: Column<Fixed>,
    // Rotation
    // 0      itable lookup
    // 1      jtable lookup
    // 2..5   mtable lookup
    // 6..9  u4 sum
    // 10..15 shared
    pub aux: Column<Advice>,

    pub u4_shared: [Column<Advice>; U4_COLUMNS],

    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableCommonConfig<F> {
    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        etable: &EventTable,
    ) -> Result<(Cell, Cell), Error> {
        let mut status_entries = Vec::with_capacity(etable.entries().len() + 1);

        // Step 1: fill fixed columns

        for i in 0..ETABLE_ROWS {
            ctx.region
                .assign_fixed(|| "etable common sel", self.sel, i, || Ok(F::one()))?;

            if i % ETABLE_STEP_SIZE == EventTableBitColumnRotation::Enable as usize {
                ctx.region.assign_fixed(
                    || "etable common block first line sel",
                    self.block_first_line_sel,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::ITableLookup as usize {
                ctx.region.assign_fixed(
                    || "itable lookup",
                    self.itable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::JTableLookup as usize {
                ctx.region.assign_fixed(
                    || "jtable lookup",
                    self.jtable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE >= EventTableUnlimitColumnRotation::MTableLookupStart as usize
                && i % ETABLE_STEP_SIZE < EventTableUnlimitColumnRotation::U64Start as usize
            {
                ctx.region.assign_fixed(
                    || "mtable lookup",
                    self.mtable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }
        }

        let mut rest_mops_cell: Option<Cell> = None;
        let mut rest_jops_cell: Option<Cell> = None;
        let mut rest_mops = etable.rest_mops();
        let mut rest_jops = etable.rest_jops();

        macro_rules! assign_advice {
            ($c:ident, $o:expr, $k:expr, $v:expr) => {
                ctx.region.assign_advice(
                    || $k,
                    self.$c,
                    ctx.offset + $o as usize,
                    || Ok(F::from($v)),
                )?
            };
        }

        // Step 2: fill Status for each eentry

        for entry in etable.entries().iter() {
            let opcode: OpcodeClass = entry.inst.opcode.clone().into();

            assign_advice!(
                shared_bits,
                EventTableBitColumnRotation::Enable,
                "shared_bits",
                1
            );

            {
                let (op_lvl1, op_lvl2) = opclass_to_two_level(opcode);

                assign_advice!(opcode_bits, op_lvl1, "opcode level 1", 1);
                assign_advice!(opcode_bits, op_lvl2, "opcode level 2", 1);
            }

            let cell = assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::RestMOps,
                "rest mops",
                rest_mops.next().unwrap()
            );
            if rest_mops_cell.is_none() {
                rest_mops_cell = Some(cell.cell());
            }

            let cell = assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::RestJOps,
                "rest jops",
                rest_jops.next().unwrap()
            );
            if rest_jops_cell.is_none() {
                rest_jops_cell = Some(cell.cell());
            }

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::EID,
                "eid",
                entry.eid
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::MOID,
                "moid",
                entry.inst.moid as u64
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::FID,
                "fid",
                entry.inst.fid as u64
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::IID,
                "iid",
                entry.inst.iid as u64
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::MMID,
                "mmid",
                entry.inst.mmid as u64
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::SP,
                "sp",
                entry.sp
            );

            assign_advice!(
                state,
                EventTableCommonRangeColumnRotation::LastJumpEid,
                "last jump eid",
                entry.last_jump_eid
            );

            // TODO: itable lookup
            /*
            ctx.region.assign_advice(
                || "itable lookup entry",
                self.aux,
                ctx.offset + EventTableUnlimitColumnRotation::ITableLookup as usize,
                || Ok(bn_to_field(&entry.inst.encode())),
            )?;
            */

            status_entries.push(Status {
                eid: entry.eid,
                moid: entry.inst.moid,
                fid: entry.inst.fid,
                iid: entry.inst.iid,
                mmid: entry.inst.mmid,
                sp: entry.sp,
                last_jump_eid: entry.last_jump_eid,
            });

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        // Step 3: fill the first disabled row

        {
            status_entries.push(Status {
                eid: 0,
                moid: 0,
                fid: 0,
                iid: 0,
                mmid: 0,
                sp: 0,
                last_jump_eid: 0,
            });

            assign_advice!(
                shared_bits,
                EventTableBitColumnRotation::Enable,
                "shared_bits",
                0
            );
        }

        // Step 4: fill lookup aux

        ctx.reset();

        for (index, entry) in etable.entries().iter().enumerate() {
            let opcode: OpcodeClass = entry.inst.opcode.clone().into();

            let step_status = StepStatus {
                current: &status_entries[index],
                next: &status_entries[index + 1],
            };

            let config = op_configs.get(&opcode).unwrap();

            config.assign(ctx, &step_status, entry)?;

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        Ok((rest_mops_cell.unwrap(), rest_jops_cell.unwrap()))
    }
}

#[derive(Clone)]
pub struct EventTableConfig<F: FieldExt> {
    common_config: EventTableCommonConfig<F>,
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

        let state = cols.next().unwrap();
        let aux = cols.next().unwrap();

        let itable_lookup = meta.fixed_column();
        let jtable_lookup = meta.fixed_column();
        let mtable_lookup = meta.fixed_column();

        let u4_shared = [0; 4].map(|_| cols.next().unwrap());

        meta.enable_equality(state);

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
            curr!(meta, state) * fixed_curr!(meta, sel)
        });

        for i in 0..U4_COLUMNS {
            rtable.configure_in_u4_range(meta, "etable u4", |meta| {
                curr!(meta, u4_shared[i]) * fixed_curr!(meta, sel)
            });
        }

        itable.configure_in_table(meta, "etable itable lookup", |meta| {
            curr!(meta, aux) * fixed_curr!(meta, itable_lookup)
        });

        mtable.configure_in_table(meta, "etable mtable lookup", |meta| {
            curr!(meta, aux) * fixed_curr!(meta, mtable_lookup)
        });

        // TODO: elegantly handle the last return
        jtable.configure_in_table(meta, "etable jtable lookup", |meta| {
            curr!(meta, aux)
                * nextn!(meta, aux, ETABLE_STEP_SIZE as i32)
                * fixed_curr!(meta, jtable_lookup)
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

        let common_config = EventTableCommonConfig {
            sel,
            block_first_line_sel,
            shared_bits,
            opcode_bits,
            state,
            itable_lookup,
            jtable_lookup,
            mtable_lookup,
            aux,
            u4_shared,
            _mark: PhantomData,
        };

        let mut op_bitmaps: BTreeMap<OpcodeClass, (i32, i32)> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

        macro_rules! configure [
            ($op:expr, $x:ident) => (
                if opcode_set.contains(&($op)) {
                    let (op_lvl1, op_lvl2) = opclass_to_two_level($op);
                    let mut allocator = EventTableCellAllocator::new(&common_config);
                    let config = $x::configure(
                        meta,
                        &mut allocator,
                        |meta| fixed_curr!(meta, common_config.block_first_line_sel)
                    );
                    op_bitmaps.insert(config.opcode_class(), (op_lvl1 as i32, op_lvl2 as i32));
                    op_configs.insert(config.opcode_class(), Rc::new(config));
                }
            )
        ];

        configure!(OpcodeClass::Return, ReturnConfigBuilder);
        configure!(OpcodeClass::Const, ConstConfigBuilder);
        configure!(OpcodeClass::Drop, DropConfigBuilder);

        meta.create_gate("enable seq", |meta| {
            vec![
                common_config.next_enable(meta)
                    * (common_config.enable(meta) - constant_from!(1))
                    * fixed_curr!(meta, common_config.block_first_line_sel),
            ]
        });

        meta.create_gate("etable common change", |meta| {
            let mut rest_mops_acc =
                common_config.next_rest_mops(meta) - common_config.rest_mops(meta);
            let mut rest_jops_acc =
                common_config.next_rest_jops(meta) - common_config.rest_jops(meta);
            let mut moid_acc = common_config.next_moid(meta) - common_config.moid(meta);
            let mut fid_acc = common_config.next_fid(meta) - common_config.fid(meta);
            let mut iid_acc =
                common_config.next_iid(meta) - common_config.iid(meta) - constant_from!(1);
            let mut sp_acc = common_config.next_sp(meta) - common_config.sp(meta);
            let mut last_jump_eid_acc =
                common_config.next_last_jump_eid(meta) - common_config.last_jump_eid(meta);

            let eid_diff =
                common_config.next_eid(meta) - common_config.eid(meta) - constant_from!(1);
            // MMID equals to MOID in single module version
            let mmid_diff = common_config.mmid(meta) - common_config.moid(meta);

            let mut itable_lookup = common_config.itable_lookup(meta);
            let mut jtable_lookup = common_config.jtable_lookup(meta);
            let mut mtable_lookup = vec![];

            for i in 0..MTABLE_LOOKUPS_SIZE {
                mtable_lookup.push(common_config.mtable_lookup(meta, i as i32));
            }

            for (op, (lvl1, lvl2)) in op_bitmaps.iter() {
                let config = op_configs.get(op).unwrap();
                match config.mops(meta) {
                    Some(e) => {
                        rest_mops_acc =
                            rest_mops_acc + e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.jops(meta) {
                    Some(e) => {
                        rest_jops_acc =
                            rest_jops_acc + e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_last_jump_eid(meta, &common_config) {
                    Some(e) => {
                        last_jump_eid_acc = last_jump_eid_acc
                            - (e - common_config.last_jump_eid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_moid(meta, &common_config) {
                    Some(e) => {
                        moid_acc = moid_acc
                            - (e - common_config.moid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_fid(meta, &common_config) {
                    Some(e) => {
                        fid_acc = fid_acc
                            - (e - common_config.fid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_iid(meta, &common_config) {
                    Some(e) => {
                        iid_acc = iid_acc
                            - (e - common_config.iid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.sp_diff(meta) {
                    Some(e) => sp_acc = sp_acc - e * common_config.op_enabled(meta, *lvl1, *lvl2),
                    _ => {}
                }

                match config.itable_lookup(meta, &common_config) {
                    Some(e) => {
                        itable_lookup =
                            itable_lookup - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.jtable_lookup(meta, &common_config) {
                    Some(e) => {
                        jtable_lookup =
                            jtable_lookup - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                for i in 0..MTABLE_LOOKUPS_SIZE {
                    match config.mtable_lookup(meta, i.try_into().unwrap(), &common_config) {
                        Some(e) => {
                            mtable_lookup[i] = mtable_lookup[i].clone()
                                - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                        }
                        _ => {}
                    }
                }
            }

            // TODO: elegantly handle the last row and then
            // delete common_config.next_enable(meta)
            vec![
                vec![
                    rest_mops_acc,
                    rest_jops_acc,
                    eid_diff * common_config.next_enable(meta),
                    moid_acc,
                    fid_acc,
                    iid_acc * common_config.next_enable(meta),
                    mmid_diff,
                    sp_acc * common_config.next_enable(meta),
                    last_jump_eid_acc,
                    itable_lookup,
                    jtable_lookup,
                ],
                mtable_lookup,
            ]
            .into_iter()
            .flatten()
            .map(|x| x * common_config.enabled_block(meta))
            .collect::<Vec<_>>()
        });

        meta.create_gate("etable op lvl bits sum", |meta| {
            let mut acc_lvl1 = constant_from!(1);
            let mut acc_lvl2 = constant_from!(1);

            for i in 0..MAX_OP_LVL1 {
                acc_lvl1 = acc_lvl1 - nextn!(meta, common_config.opcode_bits, i);
            }

            for i in MAX_OP_LVL1..ETABLE_STEP_SIZE as i32 {
                acc_lvl2 = acc_lvl2 - nextn!(meta, common_config.opcode_bits, i);
            }

            vec![acc_lvl1, acc_lvl2]
                .into_iter()
                .map(|x| x * common_config.enabled_block(meta))
                .collect::<Vec<_>>()
        });

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

    pub(super) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        etable: &EventTable,
    ) -> Result<(Cell, Cell), Error> {
        self.config
            .common_config
            .assign(ctx, &self.config.op_configs, etable)
    }
}
