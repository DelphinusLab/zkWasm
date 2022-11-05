use crate::circuits::rtable::RangeTableMixColumn;

use self::configure::JTableConstraint;
use super::config::MAX_JATBLE_ROWS;
use super::rtable::RangeTableConfig;
use super::shared_column_pool::DynTableLookupColumn;
use super::shared_column_pool::SharedColumnPool;
use super::utils::bn_to_field;
use super::utils::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
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

const JTABLE_ROWS: usize = MAX_JATBLE_ROWS / JtableOffset::JtableOffsetMax as usize
    * JtableOffset::JtableOffsetMax as usize;

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    data: DynTableLookupColumn<F>,
    _m: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        shared_column_pool: &SharedColumnPool<F>,
        rtable: &RangeTableConfig<F>,
    ) -> Self {
        let jtable = Self::new(meta, shared_column_pool);
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
        assert_eq!(
            ctx.start_offset % (JtableOffset::JtableOffsetMax as usize),
            0
        );

        for _ in 0..JTABLE_ROWS {
            if (ctx.offset as u32) % (JtableOffset::JtableOffsetMax as u32) == 0 {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "jtable sel",
                    self.config.sel,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            if (ctx.offset as u32) % (JtableOffset::JtableOffsetMax as u32) == 1 {
                self.config.data.assign_lookup(
                    &mut ctx.region.as_ref().borrow_mut(),
                    ctx.offset,
                    RangeTableMixColumn::U16,
                )?;
            }

            ctx.next();
        }

        ctx.reset();

        let entries: Vec<&JumpTableEntry> = entries.into_iter().filter(|e| e.eid != 0).collect();
        let mut rest = entries.len() as u64 * 2;
        for (i, entry) in entries.iter().enumerate() {
            let rest_f = rest.into();
            let entry_f = bn_to_field(&entry.encode());

            ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable enable",
                self.config.data.internal,
                ctx.offset,
                || Ok(F::one()),
            )?;
            ctx.next();

            let cell = ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable rest",
                self.config.data.internal,
                ctx.offset,
                || Ok(rest_f),
            )?;
            ctx.next();

            if i == 0 && etable_rest_jops_cell.is_some() {
                ctx.region
                    .as_ref()
                    .borrow_mut()
                    .constrain_equal(cell.cell(), etable_rest_jops_cell.unwrap())?;
            }

            ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable entry",
                self.config.data.internal,
                ctx.offset,
                || Ok(entry_f),
            )?;
            ctx.next();

            rest -= 2;
        }

        {
            ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable enable",
                self.config.data.internal,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();

            let cell = ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable rest",
                self.config.data.internal,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();

            if ctx.offset == ctx.start_offset && etable_rest_jops_cell.is_some() {
                ctx.region
                    .as_ref()
                    .borrow_mut()
                    .constrain_equal(cell.cell(), etable_rest_jops_cell.unwrap())?;
            }

            ctx.region.as_ref().borrow_mut().assign_advice(
                || "jtable entry",
                self.config.data.internal,
                ctx.offset,
                || Ok(F::zero()),
            )?;
            ctx.next();
        }

        Ok(())
    }
}
