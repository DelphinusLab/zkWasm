use crate::config_builder::op_const::ConstConfigBuilder;
use crate::constant_from;
use crate::curr;
use crate::itable::encode_inst_expr;
use crate::itable::Inst;
use crate::itable::InstTableConfig;
use crate::jtable::JumpTableConfig;
use crate::mtable::MemoryTableConfig;
use crate::next;
use crate::prev;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;
use wasmi::tracer::etable::EEntry;
use wasmi::tracer::etable::RunInstructionTraceStep;

pub struct Event {
    pub(crate) eid: u64,
    pub(crate) sp: u64,
    last_jump_eid: u64,
    pub(crate) inst: Inst,
    pub(crate) step_info: RunInstructionTraceStep,
}

impl From<EEntry> for Event {
    fn from(e_entry: EEntry) -> Self {
        Event {
            eid: e_entry.id,
            sp: e_entry.sp,
            // FIXME: fill with correct value
            last_jump_eid: 0,
            inst: Inst::from(e_entry.inst),
            step_info: e_entry.step,
        }
    }
}

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        itable: &InstTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
}

pub struct EventTableCommonConfig {
    pub enable: Column<Advice>,
    pub eid: Column<Advice>,
    pub moid: Column<Advice>,
    pub fid: Column<Advice>,
    pub bid: Column<Advice>,
    pub iid: Column<Advice>,
    pub mmid: Column<Advice>,
    pub sp: Column<Advice>,
    pub opcode: Column<Advice>,
}

pub struct EventTableConfig<F: FieldExt> {
    opcode_bitmaps: Vec<Column<Advice>>,
    common_config: EventTableCommonConfig,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableConfig<F> {
    pub fn new<'a>(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        itable: &InstTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
    ) -> Self {
        let enable = cols.next().unwrap();
        let eid = cols.next().unwrap();
        let moid = cols.next().unwrap();
        let fid = cols.next().unwrap();
        let bid = cols.next().unwrap();
        let iid = cols.next().unwrap();
        let mmid = cols.next().unwrap();
        let sp = cols.next().unwrap();
        let opcode = cols.next().unwrap();
        let common_config = EventTableCommonConfig {
            enable,
            eid,
            moid,
            fid,
            bid,
            iid,
            mmid,
            sp,
            opcode,
        };

        // TODO: Add opcode configures here.
        let mut opcode_bitmaps: Vec<Column<Advice>> = vec![];
        let mut opcode_bitmaps_iter = opcode_bitmaps.iter();
        let mut configs: Vec<Box<dyn EventTableOpcodeConfig<F>>> = vec![];

        {
            let opcode_bit = opcode_bitmaps_iter.next().unwrap();
            let config = ConstConfigBuilder::configure(
                meta,
                &common_config,
                opcode_bit.clone(),
                cols,
                itable,
                mtable,
                jtable,
            );
            configs.push(config);
        }

        meta.create_gate("opcode consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for config in configs.iter() {
                acc = acc + config.opcode(meta);
            }
            vec![curr!(meta, opcode) - acc]
        });

        meta.create_gate("sp diff consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for config in configs.iter() {
                acc = acc + config.sp_diff(meta);
            }
            vec![curr!(meta, sp) + acc - next!(meta, sp)]
        });

        for bit in opcode_bitmaps.iter() {
            meta.create_gate("opcode_bitmaps assert bit", |meta| {
                vec![curr!(meta, bit.clone()) * (curr!(meta, bit.clone()) - constant_from!(1u64))]
            });
        }

        meta.create_gate("opcode_bitmaps pick one", |meta| {
            vec![
                opcode_bitmaps
                    .iter()
                    .map(|x| curr!(meta, *x))
                    .reduce(|acc, x| acc + x)
                    .unwrap()
                    - constant_from!(1u64),
            ]
        });

        meta.create_gate("eid increase", |meta| {
            vec![
                curr!(meta, common_config.enable)
                    * (curr!(meta, common_config.eid)
                        - prev!(meta, common_config.eid)
                        - constant_from!(1u64)),
            ]
        });

        itable.configure_in_table(meta, "inst in table", |meta| {
            curr!(meta, enable)
                * encode_inst_expr(
                    curr!(meta, common_config.moid),
                    curr!(meta, common_config.mmid),
                    curr!(meta, common_config.fid),
                    curr!(meta, common_config.bid),
                    curr!(meta, common_config.iid),
                    curr!(meta, common_config.opcode),
                )
        });

        EventTableConfig {
            opcode_bitmaps,
            common_config,
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
}
