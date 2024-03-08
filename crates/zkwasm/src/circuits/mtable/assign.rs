use std::collections::HashMap;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Error;
use log::debug;
use specs::encode::init_memory_table::encode_init_memory_table_entry;
use specs::encode::memory_table::encode_memory_table_entry;
use specs::imtable::InitMemoryTable;
use specs::mtable::LocationType;
use specs::mtable::VarType;

use crate::circuits::cell::CellExpression;
use crate::circuits::mtable::MemoryTableChip;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::table_entry::MemoryWritingEntry;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::utils::Context;

use super::MEMORY_TABLE_ENTRY_ROWS;

impl<F: FieldExt> MemoryTableChip<F> {
    fn assign_fixed(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        let capability = self.maximal_available_rows / MEMORY_TABLE_ENTRY_ROWS as usize;

        for i in 0..capability {
            ctx.region.assign_fixed(
                || "mtable: sel",
                self.config.entry_sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if i == capability - 1 {
                ctx.region.assign_advice_from_constant(
                    || "rest_mops terminate",
                    self.config.rest_mops_cell.0.col,
                    ctx.offset + self.config.rest_mops_cell.0.rot as usize,
                    F::zero(),
                )?;
            }

            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        Ok(())
    }

    fn constraint_rest_mops_permutation(
        &self,
        ctx: &mut Context<'_, F>,
        etable_rest_mops_cell: Option<Cell>,
        init_rest_mops: u64,
    ) -> Result<AssignedCell<F, F>, Error> {
        let cell = self
            .config
            .rest_mops_cell
            .assign(ctx, F::from(init_rest_mops))?;

        if etable_rest_mops_cell.is_some() {
            ctx.region
                .constrain_equal(cell.cell(), etable_rest_mops_cell.unwrap())?;
        }

        Ok(cell)
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        mtable: &MemoryWritingTable,
        init_rest_mops: u64,
        imtable: &InitMemoryTable,
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

        let entries: Vec<MemoryWritingEntry> = mtable.0.clone();

        let entries = entries.into_iter().map(|entry| {
            let context = rest_mops;
            if !entry.entry.atype.is_init() {
                rest_mops -= 1;
            }
            (entry, context)
        }).collect::<Vec<_>>();

        let chunk_len = entries.len()/4;

        let chunks = entries.chunks(chunk_len).map(|x| (x, ctx.clone())).collect::<Vec<_>>();

        use rayon::prelude::*;
        chunks.par_iter().enumerate().map(|(index, (entries, ctx))| {
            let mut _ctx = ctx.clone();
            let ctx = &mut _ctx;
            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize * index * chunk_len);

            let _ = entries.iter().map(|(entry, rest_mops)| {

                self.config.enabled_cell.assign(ctx, F::one())?;

                match entry.entry.ltype {
                    LocationType::Stack => self.config.is_stack_cell.assign(ctx, F::one())?,
                    LocationType::Heap => self.config.is_heap_cell.assign(ctx, F::one())?,
                    LocationType::Global => self.config.is_global_cell.assign(ctx, F::one())?,
                };

                if entry.entry.is_mutable {
                    self.config.is_mutable.assign(ctx, F::one())?;
                }

                match entry.entry.vtype {
                    VarType::I32 => self.config.is_i32_cell.assign(ctx, F::one())?,
                    VarType::I64 => self.config.is_i64_cell.assign(ctx, F::one())?,
                };

                if entry.entry.atype.is_init() {
                    self.config.is_init_cell.assign(ctx, F::one())?;
                }

                if entry.entry.atype.is_init() {
                    let (left_offset, right_offset, value) = imtable
                        .try_find(entry.entry.ltype, entry.entry.offset)
                        .unwrap();

                    self.config.offset_align_left.assign(ctx, F::from(left_offset as u64))?;
                    self.config.offset_align_right.assign(ctx, F::from(right_offset as u64))?;
                    self.config.offset_align_left_diff_cell.assign(
                        ctx,
                        F::from((entry.entry.offset - left_offset) as u64)
                        )?;
                    self.config.offset_align_right_diff_cell.assign(
                        ctx,
                        F::from((right_offset - entry.entry.offset) as u64)
                        )?;

                    self.config.init_encode_cell.assign(
                        ctx,
                        bn_to_field(&encode_init_memory_table_entry(
                                (entry.entry.ltype as u64).into(),
                                (entry.entry.is_mutable as u64).into(),
                                left_offset.into(),
                                right_offset.into(),
                                value.into()
                                ))
                        )?;
                }

                self.config.start_eid_cell.assign(ctx, F::from(entry.entry.eid as u64))?;
                self.config.end_eid_cell.assign(ctx, F::from(entry.end_eid as u64))?;
                self.config.eid_diff_cell.assign(
                    ctx,
                    F::from((entry.end_eid - entry.entry.eid - 1) as u64)
                    )?;
                self.config.rest_mops_cell.assign(ctx,F::from(*rest_mops))?;
                self.config.offset_cell.assign(ctx, F::from(entry.entry.offset as u64))?;
                self.config.value.assign(ctx, entry.entry.value)?;

                self.config.encode_cell.assign(
                    ctx,
                    bn_to_field(&encode_memory_table_entry(
                            entry.entry.offset.into(),
                            (entry.entry.ltype as u64).into(),
                            if VarType::I32 == entry.entry.vtype {
                                1u64.into()
                            } else {
                                0u64.into()
                            }
                            ))
                    )?;

                ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
                Ok::<_, Error>(())
            }).collect::<Vec<_>>();
        }).collect::<Vec<_>>();

        ctx.reset();

        let mut cache = HashMap::new();
        for (curr, next) in mtable.0.iter().zip(mtable.0.iter().skip(1)) {
            if curr.entry.ltype == next.entry.ltype {
                let offset_diff = (next.entry.offset - curr.entry.offset) as u64;

                assign_bit!(is_next_same_ltype_cell);

                assign_bit_if!(
                    curr.entry.offset == next.entry.offset,
                    is_next_same_offset_cell
                );
                assign_advice!(offset_diff_cell, F::from(offset_diff));
                let invert = if let Some(f) = cache.get(&offset_diff) {
                    *f
                } else {
                    let f = F::from(offset_diff).invert().unwrap_or(F::zero());
                    cache.insert(offset_diff, f);
                    f
                };
                assign_advice!(offset_diff_inv_cell, invert);
                assign_advice!(offset_diff_inv_helper_cell, invert * F::from(offset_diff));
            }

            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        Ok(())
    }

    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        etable_rest_mops_cell: Option<Cell>,
        mtable: &MemoryWritingTable,
        imtable: &InitMemoryTable,
    ) -> Result<(), Error> {
        debug!("size of memory writing table: {}", mtable.0.len());
        assert!(mtable.0.len() * (MEMORY_TABLE_ENTRY_ROWS as usize) < self.maximal_available_rows);

        let rest_mops = mtable
            .0
            .iter()
            .fold(0, |acc, entry| acc + (!entry.entry.atype.is_init() as u64));

        self.assign_fixed(ctx)?;
        ctx.reset();

        let rest_mops_cell =
            self.constraint_rest_mops_permutation(ctx, etable_rest_mops_cell, rest_mops)?;

        /*
         * Skip subsequent advice assignment in the first pass to enhance performance.
         */
        if rest_mops_cell.value().is_some() {
            self.assign_entries(ctx, mtable, rest_mops, imtable)?;
            ctx.reset();
        }

        Ok(())
    }
}
