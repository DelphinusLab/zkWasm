use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::jtable::CalledFrameTable;
use specs::jtable::InheritedFrameTable;
use specs::jtable::INHERITED_FRAME_TABLE_ENTRIES;
use specs::slice::FrameTableSlice;
use wasmi::DEFAULT_CALL_STACK_LIMIT;

use super::FrameEtablePermutationCells;
use super::JumpTableChip;
use crate::circuits::jtable::FrameTableValueOffset;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::Context;

impl<F: FieldExt> JumpTableChip<F> {
    /// Frame Table Constraint 1. The etable and frame table: must have the same jops count."
    fn assign_first_rest_jops(
        &self,
        ctx: &mut Context<'_, F>,
        rest_call_ops: u32,
        rest_return_ops: u32,
    ) -> Result<FrameEtablePermutationCells<F>, Error> {
        assert_eq!(ctx.offset, 0);

        let assigned_rest_call_cell = ctx.region.assign_advice(
            || "frame table: rest call ops",
            self.config.value,
            ctx.offset + FrameTableValueOffset::CallOps as usize,
            || Ok(F::from(rest_call_ops as u64)),
        )?;

        let assigned_rest_return_cell = ctx.region.assign_advice(
            || "frame table: rest return ops",
            self.config.value,
            ctx.offset + FrameTableValueOffset::ReturnOps as usize,
            || Ok(F::from(rest_return_ops as u64)),
        )?;

        Ok(FrameEtablePermutationCells {
            rest_call_ops: assigned_rest_call_cell,
            rest_return_ops: assigned_rest_return_cell,
        })
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        let capability = self.max_available_rows / FrameTableValueOffset::Max as usize;

        assert_eq!(ctx.offset, 0);
        assert_eq!(INHERITED_FRAME_TABLE_ENTRIES, DEFAULT_CALL_STACK_LIMIT);
        assert!(INHERITED_FRAME_TABLE_ENTRIES < capability);

        for i in 0..capability {
            ctx.region.assign_fixed(
                || "frame table: sel",
                self.config.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if i < INHERITED_FRAME_TABLE_ENTRIES {
                ctx.region.assign_fixed(
                    || "frame table: inherited",
                    self.config.inherited,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            ctx.step(FrameTableValueOffset::Max as usize);
        }

        ctx.region.assign_advice_from_constant(
            || "frame table: entry terminate",
            self.config.value,
            ctx.offset + FrameTableValueOffset::CallOps as usize,
            F::zero(),
        )?;

        ctx.region.assign_advice_from_constant(
            || "frame table: entry terminate",
            self.config.value,
            ctx.offset + FrameTableValueOffset::ReturnOps as usize,
            F::zero(),
        )?;

        ctx.region.assign_fixed(
            || "frame table: inherited",
            self.config.inherited,
            ctx.offset,
            || Ok(F::zero()),
        )?;

        ctx.region.assign_advice(
            || "frame table: disabled row",
            self.config.value,
            ctx.offset + FrameTableValueOffset::Enable as usize,
            || Ok(F::zero()),
        )?;

        ctx.region.assign_advice(
            || "frame table: disabled row",
            self.config.value,
            ctx.offset + FrameTableValueOffset::Returned as usize,
            || Ok(F::zero()),
        )?;

        ctx.region.assign_advice(
            || "frame table: disabled row",
            self.config.value,
            ctx.offset + FrameTableValueOffset::Encode as usize,
            || Ok(F::zero()),
        )?;

        ctx.region.assign_advice(
            || "frame table: disabled row",
            self.config.value,
            ctx.offset + FrameTableValueOffset::CallOps as usize,
            || Ok(F::zero()),
        )?;

        ctx.region.assign_advice(
            || "frame table: disabled row",
            self.config.value,
            ctx.offset + FrameTableValueOffset::ReturnOps as usize,
            || Ok(F::zero()),
        )?;

        Ok(())
    }

    fn assign_inherited_entries_and_first_rest_jops(
        &self,
        ctx: &mut Context<'_, F>,
        rest_call_ops: &mut u32,
        rest_return_ops: &mut u32,
        inherited_table: &InheritedFrameTable,
    ) -> Result<Box<[AssignedCell<F, F>; INHERITED_FRAME_TABLE_ENTRIES]>, Error> {
        let mut cells = vec![];

        for entry in inherited_table.0.iter() {
            let entry_cell = ctx.region.assign_advice(
                || "frame table: encode",
                self.config.value,
                ctx.offset + FrameTableValueOffset::Encode as usize,
                || Ok(bn_to_field(&entry.encode())),
            )?;

            ctx.region.assign_advice(
                || "frame table: rest call ops",
                self.config.value,
                ctx.offset + FrameTableValueOffset::CallOps as usize,
                || Ok(F::from(*rest_call_ops as u64)),
            )?;

            ctx.region.assign_advice(
                || "frame table: rest return ops",
                self.config.value,
                ctx.offset + FrameTableValueOffset::ReturnOps as usize,
                || Ok(F::from(*rest_return_ops as u64)),
            )?;

            if let Some(entry) = entry.0.as_ref() {
                ctx.region.assign_advice(
                    || "frame table: enable",
                    self.config.value,
                    ctx.offset + FrameTableValueOffset::Enable as usize,
                    || Ok(F::one()),
                )?;

                if entry.returned {
                    ctx.region.assign_advice(
                        || "frame table: returned",
                        self.config.value,
                        ctx.offset + FrameTableValueOffset::Returned as usize,
                        || Ok(F::one()),
                    )?;

                    *rest_return_ops -= 1;
                }
            }

            cells.push(entry_cell);

            ctx.step(FrameTableValueOffset::Max as usize);
        }

        Ok(cells.try_into().unwrap_or_else(|_| {
            panic!(
                "The number of inherited frame entries should be {}",
                INHERITED_FRAME_TABLE_ENTRIES
            )
        }))
    }

    fn assign_frame_table_entries(
        &self,
        ctx: &mut Context<'_, F>,
        rest_call_ops: &mut u32,
        rest_return_ops: &mut u32,
        frame_table: &CalledFrameTable,
    ) -> Result<(), Error> {
        for entry in frame_table.iter() {
            ctx.region.assign_advice(
                || "frame table: enable",
                self.config.value,
                ctx.offset + FrameTableValueOffset::Enable as usize,
                || Ok(F::one()),
            )?;

            ctx.region.assign_advice(
                || "frame table: encode",
                self.config.value,
                ctx.offset + FrameTableValueOffset::Encode as usize,
                || Ok(bn_to_field(&entry.encode())),
            )?;

            ctx.region.assign_advice(
                || "frame table: rest call ops",
                self.config.value,
                ctx.offset + FrameTableValueOffset::CallOps as usize,
                || Ok(F::from(*rest_call_ops as u64)),
            )?;

            ctx.region.assign_advice(
                || "frame table: entry",
                self.config.value,
                ctx.offset + FrameTableValueOffset::ReturnOps as usize,
                || Ok(F::from(*rest_return_ops as u64)),
            )?;

            if entry.0.returned {
                ctx.region.assign_advice(
                    || "frame table: returned",
                    self.config.value,
                    ctx.offset + FrameTableValueOffset::Returned as usize,
                    || Ok(F::one()),
                )?;

                *rest_return_ops -= 1u32;
            }

            *rest_call_ops -= 1;

            ctx.step(FrameTableValueOffset::Max as usize);
        }

        Ok(())
    }

    fn compute_call_ops(&self, frame_table: &FrameTableSlice) -> u32 {
        frame_table.called.len() as u32
    }

    fn compute_returned_ops(&self, frame_table: &FrameTableSlice) -> u32 {
        frame_table
            .inherited
            .iter()
            .filter(|e| e.0.as_ref().map_or(false, |entry| entry.returned))
            .count() as u32
            + frame_table.called.iter().filter(|e| e.0.returned).count() as u32
    }

    pub(crate) fn assign(
        &self,
        layouter: impl Layouter<F>,
        frame_table: &FrameTableSlice,
    ) -> Result<
        (
            FrameEtablePermutationCells<F>,
            Box<[AssignedCell<F, F>; INHERITED_FRAME_TABLE_ENTRIES]>,
        ),
        Error,
    > {
        layouter.assign_region(
            || "frame table",
            |region| {
                let mut ctx = Context::new(region);

                self.init(&mut ctx)?;
                ctx.reset();

                let mut rest_call_ops = self.compute_call_ops(frame_table);
                let mut rest_return_ops = self.compute_returned_ops(frame_table);

                let frame_etable_permutation_cells =
                    self.assign_first_rest_jops(&mut ctx, rest_call_ops, rest_return_ops)?;
                let inherited_cells = self.assign_inherited_entries_and_first_rest_jops(
                    &mut ctx,
                    &mut rest_call_ops,
                    &mut rest_return_ops,
                    &frame_table.inherited,
                )?;

                self.assign_frame_table_entries(
                    &mut ctx,
                    &mut rest_call_ops,
                    &mut rest_return_ops,
                    &frame_table.called,
                )?;

                Ok((frame_etable_permutation_cells, inherited_cells))
            },
        )
    }
}
