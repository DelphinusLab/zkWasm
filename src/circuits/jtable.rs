use self::configure::JTableConstraint;

use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use specs::jtable::JumpTableEntry;
use std::marker::PhantomData;

mod configure;
mod expression;

pub enum JtableOffset {
    JtableOffsetEnable = 0,
    JtableOffsetRest = 1,
    JtableOffsetEntry = 2,
    JtableOffsetMax = 3,
}

const MAX_JATBLE_ROWS: usize = 1usize << 15;
const JTABLE_ROWS: usize = MAX_JATBLE_ROWS / JtableOffset::JtableOffsetMax as usize
    * JtableOffset::JtableOffsetMax as usize;

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    data: Column<Advice>,
    _m: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
    ) -> Self {
        let jtable = Self::new(meta, cols);
        jtable.configure(meta, rtable);
        jtable
    }
}

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
            if (i as u32) % (JtableOffset::JtableOffsetMax as u32) == 0 {
                ctx.region
                    .assign_fixed(|| "jtable sel", self.config.sel, i, || Ok(F::one()))?;
            }
        }

        let entries: Vec<&JumpTableEntry> = entries.into_iter().filter(|e| e.eid != 0).collect();
        let mut rest = entries.len() as u64 * 2;
        for (i, entry) in entries.iter().enumerate() {
            let rest_f = rest.into();
            let entry_f = bn_to_field(&entry.encode());

            ctx.region.assign_advice(
                || "jtable enable",
                self.config.data,
                ctx.offset,
                || Ok(F::one()),
            )?;
            ctx.next();

            let cell = ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok(rest_f),
            )?;
            ctx.next();

            if i == 0 && etable_rest_jops_cell.is_some() {
                ctx.region
                    .constrain_equal(cell.cell(), etable_rest_jops_cell.unwrap())?;
            }

            ctx.region.assign_advice(
                || "jtable entry",
                self.config.data,
                ctx.offset,
                || Ok(entry_f),
            )?;
            ctx.next();

            rest -= 2;
        }

        {
            ctx.region.assign_advice(
                || "jtable enable",
                self.config.data,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();

            let cell = ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();

            if ctx.offset == 0 && etable_rest_jops_cell.is_some() {
                ctx.region
                    .constrain_equal(cell.cell(), etable_rest_jops_cell.unwrap())?;
            }

            ctx.region.assign_advice(
                || "jtable entry",
                self.config.data,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();
        }

        Ok(())
    }
}
