use super::config_builder::op_const::ConstConfigBuilder;
use super::config_builder::op_drop::DropConfigBuilder;
use super::config_builder::op_local_get::LocalGetConfigBuilder;
use super::itable::encode_inst_expr;
use super::itable::InstructionTableConfig;
use super::jtable::JumpTableConfig;
use super::mtable::MemoryTableConfig;
use super::utils::Context;
use crate::circuits::utils::bn_to_field;
use crate::constant_from;
use crate::curr;
use crate::next;
use crate::prev;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error>;
    fn opcode_class(&self) -> OpcodeClass;
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
    common_config: EventTableCommonConfig,
    opcode_bitmaps: BTreeMap<OpcodeClass, Column<Advice>>,
    opcode_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        itable: &InstructionTableConfig<F>,
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

        let mut opcode_bitmaps_vec = vec![];
        let mut opcode_bitmaps = BTreeMap::new();
        let mut opcode_configs: BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>> =
            BTreeMap::new();

        macro_rules! configure [
            ($($x:ident),*) => ({
                $($x{}; opcode_bitmaps_vec.push(cols.next().unwrap());)*

                let mut opcode_bitmaps_iter = opcode_bitmaps_vec.iter();
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
                    opcode_bitmaps.insert(config.opcode_class(), opcode_bit.clone());
                    opcode_configs.insert(config.opcode_class(), Rc::new(config));
                )*
            })
        ];

        configure![ConstConfigBuilder, DropConfigBuilder, LocalGetConfigBuilder];

        meta.create_gate("opcode consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for (_, config) in opcode_configs.iter() {
                acc = acc + config.opcode(meta);
            }
            vec![curr!(meta, opcode) - acc]
        });

        meta.create_gate("sp diff consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for (_, config) in opcode_configs.iter() {
                acc = acc + config.sp_diff(meta);
            }
            vec![curr!(meta, sp) + acc - next!(meta, sp)]
        });

        for (_, bit) in opcode_bitmaps.iter() {
            meta.create_gate("opcode_bitmaps assert bit", |meta| {
                vec![curr!(meta, *bit) * (curr!(meta, *bit) - constant_from!(1u64))]
            });
        }

        meta.create_gate("opcode_bitmaps pick one", |meta| {
            vec![
                opcode_bitmaps
                    .iter()
                    .map(|(_, x)| curr!(meta, *x))
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
            common_config,
            opcode_bitmaps,
            opcode_configs,
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
    ) -> Result<(), Error> {
        for entry in entries {
            ctx.region.assign_advice(
                || "etable enable",
                self.config.common_config.enable,
                ctx.offset,
                || Ok(F::one()),
            )?;

            macro_rules! assign {
                ($x: ident, $value: expr) => {
                    ctx.region.assign_advice(
                        || concat!("etable ", stringify!($x)),
                        self.config.common_config.$x,
                        ctx.offset,
                        || Ok($value),
                    )?;
                };
            }

            macro_rules! assign_as_u64 {
                ($x: ident, $value: expr) => {
                    assign!($x, F::from($value as u64))
                };
            }

            assign_as_u64!(enable, 1u64);
            assign_as_u64!(eid, entry.eid);
            assign_as_u64!(moid, entry.inst.moid);
            assign_as_u64!(fid, entry.inst.fid);
            assign_as_u64!(bid, entry.inst.bid);
            assign_as_u64!(iid, entry.inst.iid);
            assign_as_u64!(mmid, entry.inst.mmid);
            assign_as_u64!(sp, entry.sp);
            assign!(opcode, bn_to_field(&(entry.inst.opcode.into())));

            let opcode_class = entry.inst.opcode.into();

            ctx.region.assign_advice(
                || concat!("etable opcode"),
                self.config
                    .opcode_bitmaps
                    .get(&opcode_class)
                    .unwrap()
                    .clone(),
                ctx.offset,
                || Ok(F::one()),
            )?;

            self
                .config
                .opcode_configs
                .get(&opcode_class)
                .unwrap()
                .as_ref()
                .as_ref()
                .assign(ctx, entry)?;

            ctx.next();
        }

        Ok(())
    }
}
