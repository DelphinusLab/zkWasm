use self::configure::JTableConstraint;
use super::config::max_jtable_rows;
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
pub(crate) mod expression;

pub enum JtableOffset {
    JtableOffsetEnable = 0,
    JtableOffsetRest = 1,
    JtableOffsetEntry = 2,
    JtableOffsetMax = 3,
}

fn jtable_rows() -> usize {
    max_jtable_rows() as usize / JtableOffset::JtableOffsetMax as usize
        * JtableOffset::JtableOffsetMax as usize
}

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
        for i in 0..jtable_rows() {
            if (i as u32) % (JtableOffset::JtableOffsetMax as u32) == 0 {
                ctx.region
                    .assign_fixed(|| "jtable sel", self.config.sel, i, || Ok(F::one()))?;
            }
        }

        let cell = ctx.region.assign_advice(
            || "jtable rest",
            self.config.data,
            JtableOffset::JtableOffsetRest as usize,
            || Ok(F::from(0)),
        )?;
        if let Some(etable_rest_jops_cell) = etable_rest_jops_cell {
            ctx.region
                .constrain_equal(cell.cell(), etable_rest_jops_cell)?;
        }

        let entries: Vec<&JumpTableEntry> = entries.into_iter().filter(|e| e.eid != 0).collect();
        let mut rest = entries.len() as u64 * 2;
        for entry in entries.iter() {
            let rest_f = rest.into();
            let entry_f = bn_to_field(&entry.encode());

            ctx.region.assign_advice(
                || "jtable enable",
                self.config.data,
                ctx.offset,
                || Ok(F::one()),
            )?;
            ctx.next();

            ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok(rest_f),
            )?;
            ctx.next();

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

            ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();

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
