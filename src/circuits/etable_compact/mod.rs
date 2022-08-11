use self::op_configure::EventTableOpcodeConfig;
use super::itable::Encode;
use super::*;
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

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

const ETABLE_ROWS: usize = 1usize << 16;
const ETABLE_STEP_SIZE: usize = 16usize;
const U4_COLUMNS: usize = 4usize;
const MTABLE_LOOKUPS_SIZE: usize = 4usize;

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
    U64Start = 6,
    SharedStart = 10,
}

#[derive(Clone)]
pub struct EventTableCommonConfig<F> {
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
    pub state: Column<Advice>,

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

    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        etable: &EventTable,
    ) -> Result<(Cell, Cell), Error> {
        for i in 0..ETABLE_ROWS {
            ctx.region
                .assign_fixed(|| "etable common sel", self.sel, i, || Ok(F::one()))?;

            if i % ETABLE_STEP_SIZE == 0 {
                ctx.region.assign_fixed(
                    || "etable common block first line sel",
                    self.block_first_line_sel,
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
            ($c:ident, $k:expr, $v:expr) => {
                ctx.region
                    .assign_advice(|| $k, self.$c, ctx.offset, || Ok(F::from($v)))?
            };
        }

        for entry in etable.entries().iter() {
            {
                /* Offset 0 */
                todo!(); // fill shared_bits correctly
                assign_advice!(shared_bits, "shared_bits", 0);
                assign_advice!(opcode_bits, "opcode_bits", 1);

                let cell = assign_advice!(state, "rest mops", rest_mops.next().unwrap());
                if rest_mops_cell == None {
                    rest_mops_cell = Some(cell.cell());
                }

                ctx.region.assign_fixed(
                    || "itable lookup",
                    self.itable_lookup,
                    ctx.offset,
                    || Ok(F::from(1)),
                )?;

                // from_str_vartime is ugly
                ctx.region.assign_advice(
                    || "itable lookup entry",
                    self.aux,
                    ctx.offset,
                    || Ok(F::from_str_vartime(&entry.inst.encode().to_str_radix(16)).unwrap()),
                )?;

                ctx.next();
            }

            {
                /* Offset 1 */
                let cell = assign_advice!(state, "rest jops", rest_jops.next().unwrap());
                if rest_jops_cell == None {
                    rest_jops_cell = Some(cell.cell());
                }

                ctx.next();
            }

            {
                /* Offset 2 */
                assign_advice!(state, "eid", entry.eid);

                ctx.next();
            }

            {
                /* Offset 3 */
                assign_advice!(state, "moid", entry.inst.moid as u64);

                ctx.next();
            }

            {
                /* Offset 4 */
                assign_advice!(state, "fid", entry.inst.fid as u64);

                ctx.next();
            }

            {
                /* Offset 5 */
                assign_advice!(state, "iid", entry.inst.iid as u64);

                ctx.next();
            }

            {
                /* Offset 6 */
                assign_advice!(state, "mmid", entry.inst.mmid as u64);

                ctx.next();
            }

            {
                /* Offset 7 */
                assign_advice!(state, "sp", entry.sp);

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

        jtable.configure_in_table(meta, "etable jtable lookup", |meta| {
            curr!(meta, aux) * fixed_curr!(meta, jtable_lookup)
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

        const MAX_OP_LVL1: i32 = 8;
        const MAX_OP_LVL2: i32 = ETABLE_STEP_SIZE as i32;

        let mut op_lvl1 = 0;
        let mut op_lvl2 = MAX_OP_LVL1;

        let mut op_bitmaps_vec: Vec<(i32, i32)> = vec![];
        let mut op_bitmaps: BTreeMap<OpcodeClass, (i32, i32)> = BTreeMap::new();
        let mut op_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

        macro_rules! configure [
            ($($x:ident),*) => (
                {
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
                    op_bitmaps.insert(config.opcode_class(), (op_lvl1, op_lvl2));
                    op_configs.insert(config.opcode_class(), Rc::new(config));
                )*
            })
        ];

        meta.create_gate("etable common change", |meta| {
            let mut rest_mops_acc =
                common_config.next_rest_mops(meta) - common_config.rest_mops(meta);
            let mut rest_jops_acc =
                common_config.next_rest_jops(meta) - common_config.rest_jops(meta);
            let mut moid_acc = common_config.next_moid(meta) - common_config.moid(meta);
            let mut fid_acc = common_config.next_fid(meta) - common_config.fid(meta);
            let mut iid_acc = common_config.next_iid(meta) - common_config.iid(meta);
            let mut sp_acc = common_config.next_sp(meta) - common_config.sp(meta);

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
                            rest_mops_acc - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.jops(meta) {
                    Some(e) => {
                        rest_jops_acc =
                            rest_jops_acc - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_moid(meta) {
                    Some(e) => {
                        moid_acc = moid_acc
                            - (e - common_config.moid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_fid(meta) {
                    Some(e) => {
                        fid_acc = fid_acc
                            - (e - common_config.fid(meta))
                                * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.next_iid(meta) {
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

                match config.itable_lookup(meta) {
                    Some(e) => {
                        itable_lookup =
                            itable_lookup - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                match config.jtable_lookup(meta) {
                    Some(e) => {
                        jtable_lookup =
                            jtable_lookup - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                    }
                    _ => {}
                }

                for i in 0..MTABLE_LOOKUPS_SIZE {
                    match config.mtable_lookup(meta, i as i32) {
                        Some(e) => {
                            mtable_lookup[i] = mtable_lookup[i].clone()
                                - e * common_config.op_enabled(meta, *lvl1, *lvl2)
                        }
                        _ => {}
                    }
                }
            }

            vec![
                vec![
                    rest_mops_acc,
                    rest_jops_acc,
                    eid_diff,
                    moid_acc,
                    fid_acc,
                    iid_acc,
                    mmid_diff,
                    sp_acc,
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

            for i in MAX_OP_LVL2..ETABLE_STEP_SIZE as i32 {
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

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        etable: &EventTable,
    ) -> Result<(Cell, Cell), Error> {
        self.config.common_config.assign(ctx, etable)?;

        todo!()
    }
}
