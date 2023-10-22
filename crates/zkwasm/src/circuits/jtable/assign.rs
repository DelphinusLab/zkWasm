use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Error;
use specs::jtable::JumpTable;
use specs::jtable::StaticFrameEntry;

use super::JtableOffset;
use super::JumpTableChip;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::Context;

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
        let capability = self.max_available_rows / JtableOffset::JtableOffsetMax as usize;

        for i in 0..capability {
            ctx.region.assign_fixed(
                || "jtable sel",
                self.config.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if i == capability - 1 {
                ctx.region.assign_advice_from_constant(
                    || "jtable entry terminate",
                    self.config.data,
                    ctx.offset + JtableOffset::JtableOffsetRest as usize,
                    F::zero(),
                )?;
            }

            ctx.step(JtableOffset::JtableOffsetMax as usize);
        }

        Ok(())
    }

    fn assign_static_entries(
        &self,
        ctx: &mut Context<'_, F>,
        rest_jops: &mut u64,
        static_entries: &Vec<StaticFrameEntry>,
    ) -> Result<Vec<(Cell, Cell)>, Error> {
        let mut static_entries = static_entries.clone();

        assert!(static_entries.len() == 1 || static_entries.len() == 2);

        let mut cells = vec![];

        static_entries.resize(
            2,
            StaticFrameEntry {
                enable: false,
                frame_id: 0,
                next_frame_id: 0,
                callee_fid: 0,
                fid: 0,
                iid: 0,
            },
        );

        for entry in static_entries {
            ctx.region.assign_fixed(
                || "jtable start entries",
                self.config.static_bit,
                ctx.offset,
                || Ok(F::one()),
            )?;

            let enable_cell = ctx
                .region
                .assign_advice(
                    || "jtable enable",
                    self.config.data,
                    ctx.offset,
                    || Ok(F::from(entry.enable as u64)),
                )?
                .cell();
            ctx.next();

            ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok((*rest_jops).into()),
            )?;
            ctx.next();

            let entry_cell = ctx
                .region
                .assign_advice(
                    || "jtable entry",
                    self.config.data,
                    ctx.offset,
                    || Ok(bn_to_field(&entry.encode())),
                )?
                .cell();
            ctx.next();

            cells.push((enable_cell, entry_cell));

            if entry.enable {
                *rest_jops -= 1;
            }
        }

        Ok(cells)
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
        etable_rest_jops_cell: Cell,
        static_entries: &Vec<StaticFrameEntry>,
    ) -> Result<Vec<(Cell, Cell)>, Error> {
        self.constraint_to_etable_jops(ctx, etable_rest_jops_cell)?;

        self.init(ctx)?;
        ctx.reset();

        let mut rest_jops = jtable.entries().len() as u64 * 2 + static_entries.len() as u64;

        let frame_table_start_jump_cells =
            self.assign_static_entries(ctx, &mut rest_jops, static_entries)?;
        self.assign_jtable_entries(ctx, &mut rest_jops, jtable)?;

        Ok(frame_table_start_jump_cells)
    }
}
