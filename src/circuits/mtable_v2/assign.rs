use halo2_proofs::{arithmetic::FieldExt, circuit::Cell, plonk::Error};
use specs::mtable::{LocationType, VarType};

use crate::circuits::{
    cell::CellExpression,
    mtable_v2::MemoryTableChip,
    utils::{table_entry::MemoryWritingTable, Context},
};

use super::MEMORY_TABLE_ENTRY_ROWS;

impl<F: FieldExt> MemoryTableChip<F> {
    fn assign_fixed(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for i in 0..(self.maximal_available_rows / MEMORY_TABLE_ENTRY_ROWS as usize) {
            ctx.region.assign_fixed(
                || "mtable: sel",
                self.config.entry_sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        Ok(())
    }

    fn configure_rest_mops_permutation(
        &self,
        etable_rest_mops_cell: Option<Cell>,
    ) -> Result<(), Error> {
        todo!();

        Ok(())
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        mtable: &MemoryWritingTable,
    ) -> Result<(), Error> {
        let mut rest_mops = mtable
            .0
            .iter()
            .fold(0, |acc, entry| acc + (!entry.entry.atype.is_init() as u64));

        for entry in &mtable.0 {
            self.config.enabled_cell.assign(ctx, F::one())?;

            match entry.entry.ltype {
                LocationType::Stack => self.config.is_stack_cell.assign(ctx, F::one())?,
                LocationType::Heap => self.config.is_heap_cell.assign(ctx, F::one())?,
                LocationType::Global => self.config.is_global_cell.assign(ctx, F::one())?,
            }

            self.config
                .is_mutable
                .assign(ctx, F::from(entry.entry.is_mutable))?;

            match entry.entry.vtype {
                VarType::I32 => self.config.is_i32_cell.assign(ctx, F::one())?,
                VarType::I64 => self.config.is_i64_cell.assign(ctx, F::one())?,
            }

            self.config
                .is_init_cell
                .assign(ctx, F::from(entry.entry.atype.is_init()))?;

            self.config
                .is_imtable_init_cell
                .assign(ctx, F::from(entry.entry.atype.is_positive_init()))?;

            self.config
                .start_eid_cell
                .assign(ctx, F::from(entry.entry.eid as u64))?;

            self.config
                .end_eid_cell
                .assign(ctx, F::from(entry.end_eid as u64))?;

            self.config
                .eid_diff_cell
                .assign(ctx, F::from((entry.end_eid - entry.entry.eid - 1) as u64))?;

            self.config.rest_mops_cell.assign(ctx, F::from(rest_mops))?;

            self.config
                .offset_cell
                .assign(ctx, F::from(entry.entry.offset as u64))?;

            self.config.value.assign(ctx, entry.entry.value)?;

            rest_mops -= 1;
            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        ctx.reset();

        for (curr, next) in mtable.0.iter().zip(mtable.0.iter().skip(1)) {
            if curr.entry.ltype == next.entry.ltype {
                let offset_diff = (next.entry.offset - curr.entry.offset) as u64;

                self.config.is_next_same_ltype_cell.assign(ctx, F::one())?;
                self.config
                    .is_next_same_offset_cell
                    .assign(ctx, F::from(curr.entry.offset == next.entry.offset))?;
                self.config.offset_diff.assign(ctx, F::from(offset_diff))?;
                self.config
                    .offset_diff_inv
                    .assign(ctx, F::from(offset_diff).invert().unwrap_or(F::zero()))?;
            }

            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        Ok(())
    }

    pub(crate) fn assign(
        self,
        ctx: &mut Context<'_, F>,
        etable_rest_mops_cell: Option<Cell>,
        mtable: &MemoryWritingTable,
    ) -> Result<(), Error> {
        self.assign_fixed(ctx)?;
        ctx.reset();

        self.configure_rest_mops_permutation(etable_rest_mops_cell)?;

        self.assign_entries(ctx, mtable)?;
        ctx.reset();

        Ok(())
    }
}
