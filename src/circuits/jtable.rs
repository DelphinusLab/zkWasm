use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::Context;
use super::Encode;
use crate::constant;
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
use num_bigint::BigUint;
use specs::jtable::JumpTableEntry;
use std::marker::PhantomData;
use std::vec;

impl Encode for JumpTableEntry {
    fn encode(&self) -> BigUint {
        todo!()
    }
}

const JTABLE_ROWS: usize = 1usize << 16;

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    rest: Column<Advice>,
    entry: Column<Advice>,
    aux: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
    ) -> Self {
        let sel = meta.fixed_column();
        let rest = cols.next().unwrap();
        let entry = cols.next().unwrap();
        let aux = cols.next().unwrap();

        meta.enable_equality(rest);

        meta.create_gate("jtable rest decrease", |meta| {
            vec![
                (curr!(meta, rest) - next!(meta, rest) - constant_from!(2))
                    * curr!(meta, entry)
                    * fixed_curr!(meta, sel),
            ]
        });

        // (entry == 0 -> rest == 0)
        // <-> (exists aux, entry * aux == rest)
        meta.create_gate("jtable is zero at end", |meta| {
            vec![
                (curr!(meta, entry) * curr!(meta, aux) - curr!(meta, rest))
                    * fixed_curr!(meta, sel),
            ]
        });

        rtable.configure_in_common_range(meta, "jtable rest in common range", |meta| {
            curr!(meta, rest) * fixed_curr!(meta, sel)
        });

        Self {
            sel,
            rest,
            entry,
            aux,
            _mark: PhantomData,
        }
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        last_jump_eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        moid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        fid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        iid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        let one = BigUint::from(1u64);
        meta.lookup_any("jtable lookup", |meta| {
            vec![(
                enable(meta)
                    * (eid(meta) * constant!(bn_to_field(&(&one << EID_SHIFT)))
                        + last_jump_eid(meta)
                            * constant!(bn_to_field(&(&one << LAST_JUMP_EID_SHIFT)))
                        + moid(meta) * constant!(bn_to_field(&(&one << MOID_SHIFT)))
                        + fid(meta) * constant!(bn_to_field(&(&one << FID_SHIFT)))
                        + iid(meta)),
                curr!(meta, self.entry) * fixed_curr!(meta, self.sel),
            )]
        });
    }
}

const EID_SHIFT: usize = 96;
const LAST_JUMP_EID_SHIFT: usize = 80;
const MOID_SHIFT: usize = 32;
const FID_SHIFT: usize = 32;

pub struct JumpTableChip<F: FieldExt> {
    config: JumpTableConfig<F>,
}

impl<F: FieldExt> JumpTableChip<F> {
    pub fn new(config: JumpTableConfig<F>) -> Self {
        JumpTableChip { config }
    }

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        entries: &Vec<JumpTableEntry>,
        etable_rest_jops_cell: Option<Cell>,
    ) -> Result<(), Error> {
        for i in 0..JTABLE_ROWS {
            ctx.region
                .assign_fixed(|| "jtable sel", self.config.sel, i, || Ok(F::one()))?;
        }

        let entries: Vec<&JumpTableEntry> = entries.into_iter().filter(|e| e.eid != 0).collect();
        let mut rest = entries.len() as u64 * 2;
        for (i, entry) in entries.iter().enumerate() {
            let rest_f = rest.into();
            let entry_f = bn_to_field(&entry.inst.encode_instruction_address());

            let cell = ctx.region.assign_advice(
                || "jtable rest",
                self.config.rest,
                ctx.offset,
                || Ok(rest_f),
            )?;

            if i == 0 && etable_rest_jops_cell.is_some() {
                ctx.region
                    .constrain_equal(cell.cell(), etable_rest_jops_cell.unwrap())?;
            }

            ctx.region.assign_advice(
                || "jtable entry",
                self.config.entry,
                ctx.offset,
                || Ok(entry_f),
            )?;

            ctx.region.assign_advice(
                || "jtable aux",
                self.config.aux,
                ctx.offset,
                || Ok(rest_f * entry_f.invert().unwrap()),
            )?;

            rest -= 2;
            ctx.next()
        }
        Ok(())
    }
}
