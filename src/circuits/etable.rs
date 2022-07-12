use super::config_builder::op_const::ConstConfigBuilder;
use super::config_builder::op_drop::DropConfigBuilder;
use super::config_builder::op_local_get::LocalGetConfigBuilder;
use super::itable::encode_inst_expr;
use super::itable::InstTableConfig;
use super::jtable::JumpTableConfig;
use super::mtable::MemoryTableConfig;
use crate::constant_from;
use crate::curr;
use crate::next;
use crate::prev;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;

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

#[derive(Clone)]
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

#[derive(Clone)]
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
        let mut opcode_bitmaps: Vec<Column<Advice>> = vec![cols.next().unwrap()];
        let mut configs: Vec<Box<dyn EventTableOpcodeConfig<F>>> = vec![];

        macro_rules! configure [
            ($($x:ident),*) => ({
                $($x{}; opcode_bitmaps.push(cols.next().unwrap());)*

                let mut opcode_bitmaps_iter = opcode_bitmaps.iter();
                $(
                    let opcode_bit = opcode_bitmaps_iter.next().unwrap();
                    let config = $x::configure(
                        meta,
                        &common_config,
                        opcode_bit.clone(),
                        cols,
                        itable,
                        mtable,
                        jtable,
                    );
                    configs.push(config);
                )*
            })
        ];

        configure![ConstConfigBuilder, DropConfigBuilder, LocalGetConfigBuilder];

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
