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

    fn constraint_rest_mops_permutation(
        &self,
        ctx: &mut Context<'_, F>,
        etable_rest_mops_cell: Option<Cell>,
        init_rest_mops: u64,
    ) -> Result<(), Error> {
        let cell = self
            .config
            .rest_mops_cell
            .assign(ctx, F::from(init_rest_mops))?;

        ctx.region
            .constrain_equal(cell.cell(), etable_rest_mops_cell.unwrap())?;

        Ok(())
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        mtable: &MemoryWritingTable,
        init_rest_mops: u64,
    ) -> Result<(), Error> {
        macro_rules! assign_advice {
            ($cell:ident, $value:expr) => {
                self.config.$cell.assign(ctx, $value)?
            };
        }

        macro_rules! assign_bit {
            ($cell:ident) => {
                assign_advice!($cell, F::one())
            };
        }

        macro_rules! assign_bit_if {
            ($cond:expr, $cell:ident) => {
                if $cond {
                    assign_advice!($cell, F::one());
                }
            };
        }

        let mut rest_mops = init_rest_mops;

        for entry in &mtable.0 {
            assign_bit!(enabled_cell);

            match entry.entry.ltype {
                LocationType::Stack => assign_bit!(is_stack_cell),
                LocationType::Heap => assign_bit!(is_heap_cell),
                LocationType::Global => assign_bit!(is_global_cell),
            };

            assign_bit_if!(entry.entry.is_mutable, is_mutable);

            match entry.entry.vtype {
                VarType::I32 => assign_bit!(is_i32_cell),
                VarType::I64 => assign_bit!(is_i64_cell),
            };

            assign_bit_if!(entry.entry.atype.is_init(), is_init_cell);
            assign_bit_if!(entry.entry.atype.is_positive_init(), is_imtable_init_cell);

            assign_advice!(start_eid_cell, F::from(entry.entry.eid as u64));
            assign_advice!(end_eid_cell, F::from(entry.end_eid as u64));
            assign_advice!(
                eid_diff_cell,
                F::from((entry.end_eid - entry.entry.eid - 1) as u64)
            );
            assign_advice!(rest_mops_cell, F::from(rest_mops));
            assign_advice!(offset_cell, F::from(entry.entry.offset as u64));
            assign_advice!(value, entry.entry.value);

            rest_mops -= 1;
            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        ctx.reset();

        for (curr, next) in mtable.0.iter().zip(mtable.0.iter().skip(1)) {
            if curr.entry.ltype == next.entry.ltype {
                let offset_diff = (next.entry.offset - curr.entry.offset) as u64;

                assign_bit!(is_next_same_ltype_cell);

                assign_bit_if!(
                    curr.entry.offset == next.entry.offset,
                    is_next_same_offset_cell
                );
                assign_advice!(offset_diff, F::from(offset_diff));
                assign_advice!(
                    offset_diff_inv,
                    F::from(offset_diff).invert().unwrap_or(F::zero())
                );
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
        let mut rest_mops = mtable
            .0
            .iter()
            .fold(0, |acc, entry| acc + (!entry.entry.atype.is_init() as u64));

        self.assign_fixed(ctx)?;
        ctx.reset();

        self.constraint_rest_mops_permutation(ctx, etable_rest_mops_cell, rest_mops)?;

        self.assign_entries(ctx, mtable, rest_mops)?;
        ctx.reset();

        Ok(())
    }
}
