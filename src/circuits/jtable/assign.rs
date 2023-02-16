use halo2_proofs::{arithmetic::FieldExt, circuit::Cell, plonk::Error};
use specs::jtable::{JumpTable, StaticFrameEntry};

use super::{jtable_rows, JtableOffset, JumpTableChip};
use crate::circuits::utils::{bn_to_field, Context};

impl<F: FieldExt> JumpTableChip<F> {
    /// Frame Table Constraint 1. The etable and jtable must have the same jops count."
    fn constraint_to_etable_jops(
        &self,
        ctx: &mut Context<'_, F>,
        etable_rest_jops_cell: Cell,
    ) -> Result<(), Error> {
        /*
         * Assign a meaningless rest_jops value to get the cell which should equal to the
         * rest_jops cell in etable.
         *
         * The value will be overwritten in assign_static_entries/assign_jtable_entries.
         */
        let cell = ctx.region.assign_advice(
            || "jtable rest",
            self.config.data,
            JtableOffset::JtableOffsetRest as usize,
            || Ok(F::from(0)),
        )?;

        ctx.region
            .constrain_equal(cell.cell(), etable_rest_jops_cell)?;

        Ok(())
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for i in 0..jtable_rows() {
            if (i as u32) % (JtableOffset::JtableOffsetMax as u32) == 0 {
                ctx.region
                    .assign_fixed(|| "jtable sel", self.config.sel, i, || Ok(F::one()))?;
            }
        }

        Ok(())
    }

    fn assign_static_entries(
        &self,
        ctx: &mut Context<'_, F>,
        rest_jops: &mut u64,
        static_entries: &Vec<StaticFrameEntry>,
    ) -> Result<(), Error> {
        for entry in static_entries {
            let rest_f = (*rest_jops).into();
            let entry_f = bn_to_field(&entry.encode());

            ctx.region.assign_fixed(
                || "jtable static entry",
                self.config.static_bit,
                ctx.offset,
                || Ok(F::one()),
            )?;

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

            // Frame Table Constraint 2. Static entry must be a constant."
            ctx.region.assign_advice_from_constant(
                || "jtable entry",
                self.config.data,
                ctx.offset,
                entry_f,
            )?;
            ctx.next();

            *rest_jops -= 1;
        }

        Ok(())
    }

    fn assign_jtable_entries(
        &self,
        ctx: &mut Context<'_, F>,
        rest_jops: &mut u64,
        jtable: &JumpTable,
    ) -> Result<(), Error> {
        for entry in jtable.entries().iter() {
            let rest_f = (*rest_jops).into();
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

            *rest_jops -= 2;
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

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        jtable: &JumpTable,
        etable_rest_jops_cell: Option<Cell>,
        static_entries: &Vec<StaticFrameEntry>,
    ) -> Result<(), Error> {
        if etable_rest_jops_cell.is_some() {
            self.constraint_to_etable_jops(ctx, etable_rest_jops_cell.unwrap())?;
        }

        self.init(ctx)?;

        let mut rest_jops = jtable.entries().len() as u64 * 2 + static_entries.len() as u64;

        self.assign_static_entries(ctx, &mut rest_jops, static_entries)?;
        self.assign_jtable_entries(ctx, &mut rest_jops, jtable)?;

        Ok(())
    }
}
