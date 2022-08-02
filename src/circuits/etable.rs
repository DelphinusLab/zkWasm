use super::config_builder::op_const::ConstConfigBuilder;
use super::config_builder::op_drop::DropConfigBuilder;
use super::config_builder::op_local_get::LocalGetConfigBuilder;
use super::itable::encode_inst_expr;
use super::itable::InstructionTableConfig;
use super::jtable::JumpTableConfig;
use super::mtable::MemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::utils::Context;
use crate::circuits::config_builder::op_bin::BinOpConfigBuilder;
use crate::circuits::config_builder::op_bin_bit::BinBitOpConfigBuilder;
use crate::circuits::config_builder::op_br::BrConfigBuilder;
use crate::circuits::config_builder::op_br_if::BrIfConfigBuilder;
use crate::circuits::config_builder::op_call::CallConfigBuilder;
use crate::circuits::config_builder::op_host_time::CallHostTimeConfigBuilder;
use crate::circuits::config_builder::op_load::LoadConfigBuilder;
use crate::circuits::config_builder::op_local_set::LocalSetConfigBuilder;
use crate::circuits::config_builder::op_local_tee::LocalTeeConfigBuilder;
use crate::circuits::config_builder::op_rel::RelOpConfigBuilder;
use crate::circuits::config_builder::op_return::ReturnConfigBuilder;
use crate::circuits::config_builder::op_shift::BinShiftOpConfigBuilder;
use crate::circuits::config_builder::op_store::StoreConfigBuilder;
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

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
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

#[derive(Clone)]
pub struct EventTableCommonConfig {
    pub sel: Column<Fixed>,
    pub enable: Column<Advice>,
    pub rest_mops: Column<Advice>,
    pub rest_jops: Column<Advice>,
    pub eid: Column<Advice>,
    pub moid: Column<Advice>,
    pub fid: Column<Advice>,
    pub iid: Column<Advice>,
    pub mmid: Column<Advice>,
    pub sp: Column<Advice>,
    pub opcode: Column<Advice>,
    pub last_jump_eid: Column<Advice>,
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
        let sel = meta.fixed_column();
        let enable = cols.next().unwrap();
        let eid = cols.next().unwrap();
        let moid = cols.next().unwrap();
        let fid = cols.next().unwrap();
        let iid = cols.next().unwrap();
        let mmid = cols.next().unwrap();
        let sp = cols.next().unwrap();
        let opcode = cols.next().unwrap();
        let rest_mops = cols.next().unwrap();
        let rest_jops = cols.next().unwrap();
        let last_jump_eid = cols.next().unwrap();

        meta.enable_equality(rest_mops);
        meta.enable_equality(rest_jops);

        let common_config = EventTableCommonConfig {
            sel,
            rest_mops,
            rest_jops,
            enable,
            eid,
            moid,
            fid,
            iid,
            mmid,
            sp,
            opcode,
            last_jump_eid,
        };

        let mut opcode_bitmaps_vec: Vec<Column<Advice>> = vec![];
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
                        &mut cols.clone(),
                        rtable,
                        itable,
                        mtable,
                        jtable,
                        |meta| fixed_curr!(meta, sel)
                    );
                    opcode_bitmaps.insert(config.opcode_class(), opcode_bit.clone());
                    opcode_configs.insert(config.opcode_class(), Rc::new(config));
                )*
            })
        ];

        configure![
            ConstConfigBuilder,
            DropConfigBuilder,
            LocalGetConfigBuilder,
            LocalSetConfigBuilder,
            LocalTeeConfigBuilder,
            ReturnConfigBuilder,
            BinOpConfigBuilder,
            BinBitOpConfigBuilder,
            BinShiftOpConfigBuilder,
            RelOpConfigBuilder,
            BrConfigBuilder,
            BrIfConfigBuilder,
            CallConfigBuilder,
            CallHostTimeConfigBuilder,
            LoadConfigBuilder,
            StoreConfigBuilder
        ];

        meta.create_gate("opcode consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for (opcode_class, config) in opcode_configs.iter() {
                acc = acc
                    + config.opcode(meta) * curr!(meta, *opcode_bitmaps.get(opcode_class).unwrap());
            }
            vec![(curr!(meta, opcode) - acc) * fixed_curr!(meta, common_config.sel)]
        });

        meta.create_gate("sp diff consistent", |meta| {
            let mut acc = constant_from!(0u64);
            for (opcode_class, config) in opcode_configs.iter() {
                acc = acc
                    + config.sp_diff(meta)
                        * curr!(meta, *opcode_bitmaps.get(opcode_class).unwrap());
            }
            vec![
                (curr!(meta, sp) + acc - next!(meta, sp))
                    * next!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        for (_, bit) in opcode_bitmaps.iter() {
            meta.create_gate("opcode_bitmaps assert bit", |meta| {
                vec![
                    (curr!(meta, *bit) * (curr!(meta, *bit) - constant_from!(1u64)))
                        * fixed_curr!(meta, common_config.sel),
                ]
            });
        }

        meta.create_gate("opcode_bitmaps pick one", |meta| {
            vec![
                (opcode_bitmaps
                    .iter()
                    .map(|(_, x)| curr!(meta, *x))
                    .reduce(|acc, x| acc + x)
                    .unwrap()
                    - constant_from!(1u64))
                    * curr!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("eid increase", |meta| {
            vec![
                next!(meta, common_config.enable)
                    * (next!(meta, common_config.eid)
                        - curr!(meta, common_config.eid)
                        - constant_from!(1u64))
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        itable.configure_in_table(meta, "inst in table", |meta| {
            curr!(meta, enable)
                * encode_inst_expr(
                    curr!(meta, common_config.moid),
                    curr!(meta, common_config.mmid),
                    curr!(meta, common_config.fid),
                    curr!(meta, common_config.iid),
                    curr!(meta, common_config.opcode),
                )
                * fixed_curr!(meta, common_config.sel)
        });

        meta.create_gate("rest_mops decrease", |meta| {
            let curr_mops = opcode_bitmaps
                .iter()
                .map(|(opcode_class, x)| {
                    curr!(meta, *x)
                        * (constant_from!(opcode_class.mops())
                            + opcode_configs.get(opcode_class).unwrap().extra_mops(meta))
                })
                .reduce(|acc, x| acc + x)
                .unwrap();
            vec![
                curr!(meta, common_config.enable)
                    * (curr!(meta, common_config.rest_mops)
                        - next!(meta, common_config.rest_mops)
                        - curr_mops)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("rest_mops is zero at end", |meta| {
            vec![
                (curr!(meta, common_config.enable) - constant_from!(1))
                    * curr!(meta, common_config.rest_mops)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("rest_jops decrease", |meta| {
            let curr_mops = opcode_bitmaps
                .iter()
                .map(|(opcode_class, x)| curr!(meta, *x) * constant_from!(opcode_class.jops()))
                .reduce(|acc, x| acc + x)
                .unwrap();
            vec![
                curr!(meta, common_config.enable)
                    * (curr!(meta, common_config.rest_jops)
                        - next!(meta, common_config.rest_jops)
                        - curr_mops * next!(meta, common_config.enable))
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("rest_jops is zero at end", |meta| {
            vec![
                (curr!(meta, common_config.enable) - constant_from!(1))
                    * curr!(meta, common_config.rest_mops)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("enable is bit", |meta| {
            vec![
                (curr!(meta, common_config.enable) - constant_from!(1))
                    * curr!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel),
            ]
        });

        meta.create_gate("next inst addr", |meta| {
            let mut moid_bit_acc = constant_from!(1);
            let mut fid_bit_acc = constant_from!(1);
            let mut iid_bit_acc = constant_from!(1);
            for op in OpcodeClass::iter() {
                if let Some(x) = opcode_bitmaps.get(&op) {
                    if let Some(config) = opcode_configs.get(&op) {
                        if config.handle_moid() {
                            moid_bit_acc = moid_bit_acc - curr!(meta, *x);
                        }
                        if config.handle_fid() {
                            fid_bit_acc = fid_bit_acc - curr!(meta, *x);
                        }
                        if config.handle_iid() {
                            iid_bit_acc = iid_bit_acc - curr!(meta, *x);
                        }
                    }
                }
            }

            vec![
                next!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel)
                    * (next!(meta, common_config.moid) - curr!(meta, common_config.moid))
                    * moid_bit_acc.clone(),
                next!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel)
                    * (next!(meta, common_config.fid) - curr!(meta, common_config.fid))
                    * fid_bit_acc.clone(),
                next!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel)
                    * (next!(meta, common_config.iid)
                        - curr!(meta, common_config.iid)
                        - constant_from!(1))
                    * iid_bit_acc.clone(),
            ]
        });

        meta.create_gate("next inst addr", |meta| {
            let mut acc = constant_from!(1);
            for op in OpcodeClass::iter() {
                if let Some(x) = opcode_bitmaps.get(&op) {
                    if let Some(config) = opcode_configs.get(&op) {
                        if config.last_jump_eid_change() {
                            acc = acc - curr!(meta, *x);
                        }
                    }
                }
            }

            vec![
                next!(meta, common_config.enable)
                    * fixed_curr!(meta, common_config.sel)
                    * (next!(meta, common_config.last_jump_eid)
                        - curr!(meta, common_config.last_jump_eid))
                    * acc,
            ]
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
    ) -> Result<(Cell, Cell), Error> {
        for i in 0..ETABLE_ROWS {
            ctx.region.assign_fixed(
                || "etable sel",
                self.config.common_config.sel,
                i,
                || Ok(F::one()),
            )?;
        }

        let mut rest_mops_cell_opt = None;
        let mut rest_mops = entries.iter().fold(0, |acc, entry| {
            acc + entry.extra_mops() + entry.inst.opcode.mops()
        });

        let mut rest_jops_cell_opt = None;
        // minus 1 becuase the last return is not a jump
        let mut rest_jops = entries
            .iter()
            .fold(0, |acc, entry| acc + entry.inst.opcode.jops())
            - 1;

        for (i, entry) in entries.into_iter().enumerate() {
            macro_rules! assign {
                ($x: ident, $value: expr) => {{
                    let cell = ctx.region.assign_advice(
                        || concat!("etable ", stringify!($x)),
                        self.config.common_config.$x,
                        ctx.offset,
                        || Ok($value),
                    )?;
                    cell
                }};
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
            assign_as_u64!(iid, entry.inst.iid);
            assign_as_u64!(mmid, entry.inst.mmid);
            assign_as_u64!(sp, entry.sp);
            assign_as_u64!(last_jump_eid, entry.last_jump_eid);
            assign!(opcode, bn_to_field(&(entry.inst.opcode.clone().into())));

            let opcode_class = entry.inst.opcode.clone().into();

            for (key, cols) in self.config.opcode_bitmaps.iter() {
                ctx.region.assign_advice(
                    || concat!("etable opcode"),
                    cols.clone(),
                    ctx.offset,
                    || {
                        Ok(if *key == entry.inst.opcode.clone().into() {
                            F::one()
                        } else {
                            F::zero()
                        })
                    },
                )?;
            }

            self.config
                .opcode_configs
                .get(&opcode_class)
                .unwrap()
                .as_ref()
                .as_ref()
                .assign(ctx, entry)?;

            let rest_mops_cell = assign_as_u64!(rest_mops, rest_mops);
            let rest_jops_cell = assign_as_u64!(rest_jops, rest_jops);
            if i == 0 {
                rest_mops_cell_opt = Some(rest_mops_cell.cell());
                rest_jops_cell_opt = Some(rest_jops_cell.cell());
            }

            rest_mops -= entry.inst.opcode.mops() + entry.extra_mops();
            if rest_jops > 0 {
                rest_jops -= entry.inst.opcode.jops();
            }
            ctx.next();
        }

        Ok((rest_mops_cell_opt.unwrap(), rest_jops_cell_opt.unwrap()))
    }
}
