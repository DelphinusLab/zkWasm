use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::jtable::JumpTable;
use specs::jtable::StaticFrameEntry;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;

use super::JtableOffset;
use super::JumpTableChip;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::Context;

impl<F: FieldExt> JumpTableChip<F> {
    /// Frame Table Constraint 1. The etable and jtable must have the same jops count."
    fn assign_first_rest_jops(
        &self,
        ctx: &mut Context<'_, F>,
        rest_jops: u64,
    ) -> Result<AssignedCell<F, F>, Error> {
        let cell = ctx.region.assign_advice(
            || "jtable rest",
            self.config.data,
            JtableOffset::JtableOffsetRest as usize,
            || Ok(F::from(rest_jops as u64)),
        )?;

        Ok(cell)
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

    fn assign_static_entries_and_first_rest_jops(
        &self,
        ctx: &mut Context<'_, F>,
        rest_jops: &mut u64,
        static_entries: &[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER],
    ) -> Result<[(AssignedCell<F, F>, AssignedCell<F, F>); STATIC_FRAME_ENTRY_NUMBER], Error> {
        let mut cells = vec![];

        for entry in static_entries {
            ctx.region.assign_fixed(
                || "jtable start entries",
                self.config.static_bit,
                ctx.offset,
                || Ok(F::one()),
            )?;

            let enable_cell = ctx.region.assign_advice(
                || "jtable enable",
                self.config.data,
                ctx.offset,
                || Ok(F::from(entry.enable as u64)),
            )?;
            ctx.next();

            ctx.region.assign_advice(
                || "jtable rest",
                self.config.data,
                ctx.offset,
                || Ok((*rest_jops).into()),
            )?;
            ctx.next();

            let entry_cell = ctx.region.assign_advice(
                || "jtable entry",
                self.config.data,
                ctx.offset,
                || Ok(bn_to_field(&entry.encode())),
            )?;
            ctx.next();

            cells.push((enable_cell, entry_cell));

            if entry.enable {
                *rest_jops -= 1;
            }
        }

        Ok(cells.try_into().expect(&format!(
            "The number of static frame entries should be {}",
            STATIC_FRAME_ENTRY_NUMBER
        )))
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
        layouter: impl Layouter<F>,
        static_entries: &[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER],
        jtable: &JumpTable,
    ) -> Result<
        (
            AssignedCell<F, F>,
            [(AssignedCell<F, F>, AssignedCell<F, F>); STATIC_FRAME_ENTRY_NUMBER],
        ),
        Error,
    > {
        layouter.assign_region(
            || "frame table",
            |region| {
                let mut ctx = Context::new(region);

                self.init(&mut ctx)?;
                ctx.reset();

                // non-static entry includes `call`` and `return`` op, static entry only includes `return` op
                let mut rest_jops = jtable.entries().len() as u64 * 2
                    + static_entries.iter().filter(|entry| entry.enable).count() as u64;

                let rest_jopss = self.assign_first_rest_jops(&mut ctx, rest_jops)?;
                let cells_to_permutation = self.assign_static_entries_and_first_rest_jops(
                    &mut ctx,
                    &mut rest_jops,
                    static_entries,
                )?;
                self.assign_jtable_entries(&mut ctx, &mut rest_jops, jtable)?;

                Ok((rest_jopss, cells_to_permutation))
            },
        )
    }
}
