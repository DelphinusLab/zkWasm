use std::collections::HashMap;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::Region;
use halo2_proofs::plonk::Error;
use log::debug;
use num_bigint::BigUint;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelSlice;
use specs::encode::init_memory_table::encode_init_memory_table_address;
use specs::encode::init_memory_table::encode_init_memory_table_entry;
use specs::encode::init_memory_table::MEMORY_ADDRESS_OFFSET;
use specs::encode::memory_table::encode_memory_table_entry;
use specs::mtable::AccessType;
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
                    self.config.rest_mops_cell.cell.col,
                    ctx.offset + self.config.rest_mops_cell.cell.rot as usize,
                    F::zero(),
                )?;
            }

            ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
        }

        ctx.region.assign_advice_from_constant(
            || "rest_mops terminate",
            self.config.rest_mops_cell.cell.col,
            ctx.offset + self.config.rest_mops_cell.cell.rot as usize,
            F::zero(),
        )?;

        #[cfg(feature = "continuation")]
        ctx.region.assign_advice_from_constant(
            || "rest_memory_finalize_ops terminate",
            self.config.rest_memory_finalize_ops_cell.cell.col,
            ctx.offset + self.config.rest_memory_finalize_ops_cell.cell.rot as usize,
            F::zero(),
        )?;

        Ok(())
    }

    #[cfg(feature = "continuation")]
    fn constrain_rest_memory_finalize_ops(
        &self,
        ctx: &mut Context<'_, F>,
        rest_memory_finalize_ops: u32,
    ) -> Result<AssignedCell<F, F>, Error> {
        let cell = self
            .config
            .rest_memory_finalize_ops_cell
            .assign(ctx, F::from(rest_memory_finalize_ops as u64))?;

        Ok(cell)
    }

    fn constrain_rest_mops_permutation(
        &self,
        ctx: &mut Context<'_, F>,
        init_rest_mops: u64,
    ) -> Result<AssignedCell<F, F>, Error> {
        let cell = self
            .config
            .rest_mops_cell
            .assign(ctx, F::from(init_rest_mops))?;

        Ok(cell)
    }

    fn assign_entries(
        &self,
        region: &Region<'_, F>,
        mtable: &MemoryWritingTable,
        init_rest_mops: u64,
        _rest_memory_finalize_ops: u32,
    ) -> Result<(), Error> {
        macro_rules! assign_advice {
            ($ctx:expr, $cell:ident, $value:expr) => {
                self.config.$cell.assign($ctx, $value).unwrap()
            };
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "continuation")] {
                macro_rules! assign_u32_state {
                    ($ctx:expr, $cell:ident, $value:expr) => {
                        self.config.$cell.assign($ctx, $value).unwrap()
                    }
                }
            } else {
                macro_rules! assign_u32_state {
                    ($ctx:expr, $cell:ident, $value:expr) => {
                        assign_advice!($ctx, $cell, F::from($value as u64))
                    }
                }
            }
        }

        macro_rules! assign_bit {
            ($ctx:expr, $cell:ident) => {
                assign_advice!($ctx, $cell, F::one())
            };
        }

        macro_rules! assign_bit_if {
            ($ctx:expr, $cond:expr, $cell:ident) => {
                if $cond {
                    assign_advice!($ctx, $cell, F::one());
                }
            };
        }

        struct Status<F: FieldExt> {
            rest_mops: u64,

            init_encode: F,

            is_next_same_ltype_cell: bool,
            is_next_same_offset_cell: bool,
            offset_diff: u32,

            _rest_memory_finalize_ops: u32,
            _post_init_encode_cell: Option<F>,
        }

        let status = {
            let mut status = Vec::with_capacity(mtable.0.len());

            let mut rest_mops = init_rest_mops;
            let mut _rest_memory_finalize_ops = _rest_memory_finalize_ops;
            let mut current_address_init_encode = None;

            let mut iter = mtable.0.iter().peekable();

            let is_finalized_writing_entry =
                |entry: &MemoryWritingEntry, next_entry: Option<&&MemoryWritingEntry>| {
                    entry.entry.atype == AccessType::Write
                        && (next_entry.is_none()
                            || !next_entry
                                .as_ref()
                                .unwrap()
                                .entry
                                .is_same_location(&entry.entry))
                };

            while let Some(curr) = iter.next() {
                let next = iter.peek();

                if curr.entry.atype.is_init() {
                    current_address_init_encode =
                        Some(bn_to_field(&encode_init_memory_table_entry(
                            (curr.entry.ltype as u64).into(),
                            curr.entry.offset.into(),
                            (curr.entry.is_mutable as u64).into(),
                            curr.entry.eid.into(),
                            curr.entry.value.into(),
                        )));
                }

                let (is_next_same_ltype_cell, is_next_same_offset_cell, offset_diff) =
                    if let Some(next) = next {
                        if curr.entry.ltype == next.entry.ltype {
                            let offset_diff = next.entry.offset - curr.entry.offset;

                            (true, curr.entry.offset == next.entry.offset, offset_diff)
                        } else {
                            (false, false, 0u32)
                        }
                    } else {
                        (false, false, 0u32)
                    };

                status.push(Status {
                    rest_mops,

                    init_encode: current_address_init_encode.unwrap_or(F::zero()),

                    is_next_same_ltype_cell,
                    is_next_same_offset_cell,
                    offset_diff,

                    _rest_memory_finalize_ops,
                    _post_init_encode_cell: if is_finalized_writing_entry(curr, next) {
                        Some(bn_to_field(
                            &((encode_init_memory_table_address::<BigUint>(
                                (curr.entry.ltype as u64).into(),
                                curr.entry.offset.into(),
                            )) * MEMORY_ADDRESS_OFFSET
                                + (encode_init_memory_table_entry::<BigUint>(
                                    (curr.entry.ltype as u64).into(),
                                    curr.entry.offset.into(),
                                    (curr.entry.is_mutable as u64).into(),
                                    curr.entry.eid.into(),
                                    curr.entry.value.into(),
                                ))),
                        ))
                    } else {
                        None
                    },
                });

                if let Some(next_entry) = next {
                    if !next_entry.entry.is_same_location(&curr.entry) {
                        current_address_init_encode = None;
                    }
                }

                if is_finalized_writing_entry(curr, next) {
                    _rest_memory_finalize_ops -= 1;
                }

                if !curr.entry.atype.is_init() {
                    rest_mops -= 1;
                }
            }

            status
        };

        const THREAD: usize = 8;
        let chunk_size = if mtable.0.is_empty() {
            1
        } else {
            (mtable.0.len() + THREAD - 1) / THREAD
        };

        mtable
            .0
            .par_chunks(chunk_size)
            .enumerate()
            .for_each(|(chunk_index, entries)| {
                let mut ctx = Context::new(region);
                ctx.offset = (chunk_index * chunk_size) * MEMORY_TABLE_ENTRY_ROWS as usize;
                let mut invert_cache: HashMap<u64, F> = HashMap::default();

                for (index, entry) in entries.iter().enumerate() {
                    let index = chunk_index * chunk_size + index;

                    assign_bit!(&mut ctx, enabled_cell);

                    match entry.entry.ltype {
                        LocationType::Stack => assign_bit!(&mut ctx, is_stack_cell),
                        LocationType::Heap => assign_bit!(&mut ctx, is_heap_cell),
                        LocationType::Global => assign_bit!(&mut ctx, is_global_cell),
                    };

                    assign_bit_if!(&mut ctx, entry.entry.is_mutable, is_mutable);

                    match entry.entry.vtype {
                        VarType::I32 => assign_bit!(&mut ctx, is_i32_cell),
                        VarType::I64 => assign_bit!(&mut ctx, is_i64_cell),
                    };

                    assign_bit_if!(&mut ctx, entry.entry.atype.is_init(), is_init_cell);

                    assign_u32_state!(&mut ctx, start_eid_cell, entry.entry.eid);
                    assign_u32_state!(&mut ctx, end_eid_cell, entry.end_eid);
                    assign_u32_state!(&mut ctx, eid_diff_cell, entry.end_eid - entry.entry.eid - 1);
                    assign_advice!(&mut ctx, init_encode_cell, status[index].init_encode);
                    assign_advice!(&mut ctx, rest_mops_cell, F::from(status[index].rest_mops));
                    assign_advice!(&mut ctx, offset_cell, entry.entry.offset);
                    assign_advice!(&mut ctx, value, entry.entry.value);

                    let offset_diff = F::from(status[index].offset_diff as u64);
                    let offset_diff_inv = invert_cache
                        .entry(status[index].offset_diff as u64)
                        .or_insert_with(|| offset_diff.invert().unwrap_or(F::zero()));
                    let offset_diff_inv_helper = if status[index].offset_diff == 0 {
                        F::zero()
                    } else {
                        F::one()
                    };

                    assign_bit_if!(
                        &mut ctx,
                        status[index].is_next_same_ltype_cell,
                        is_next_same_ltype_cell
                    );
                    assign_bit_if!(
                        &mut ctx,
                        status[index].is_next_same_offset_cell,
                        is_next_same_offset_cell
                    );
                    assign_advice!(&mut ctx, offset_diff_cell, status[index].offset_diff);
                    assign_advice!(&mut ctx, offset_diff_inv_cell, *offset_diff_inv);
                    assign_advice!(
                        &mut ctx,
                        offset_diff_inv_helper_cell,
                        offset_diff_inv_helper
                    );

                    #[cfg(feature = "continuation")]
                    {
                        assign_advice!(
                            &mut ctx,
                            rest_memory_finalize_ops_cell,
                            F::from(status[index]._rest_memory_finalize_ops as u64)
                        );

                        assign_advice!(
                            &mut ctx,
                            address_encode_cell,
                            bn_to_field(&encode_init_memory_table_address(
                                (entry.entry.ltype as u64).into(),
                                entry.entry.offset.into()
                            ))
                        );

                        if let Some(post_init_encode) = status[index]._post_init_encode_cell {
                            assign_advice!(&mut ctx, post_init_encode_cell, post_init_encode);
                        }
                    }

                    assign_advice!(
                        &mut ctx,
                        encode_cell,
                        bn_to_field(&encode_memory_table_entry(
                            entry.entry.offset.into(),
                            (entry.entry.ltype as u64).into(),
                            if VarType::I32 == entry.entry.vtype {
                                1u64.into()
                            } else {
                                0u64.into()
                            }
                        ))
                    );

                    ctx.step(MEMORY_TABLE_ENTRY_ROWS as usize);
                }
            });

        Ok(())
    }

    pub(crate) fn assign(
        &self,
        layouter: impl Layouter<F>,
        mtable: &MemoryWritingTable,
    ) -> Result<(AssignedCell<F, F>, Option<AssignedCell<F, F>>), Error> {
        layouter.assign_region(
            || "mtable",
            |region| {
                let mut ctx = Context::new(region);

                debug!("size of memory writing table: {}", mtable.0.len());
                assert!(
                    mtable.0.len() * (MEMORY_TABLE_ENTRY_ROWS as usize)
                        < self.maximal_available_rows
                );

                let rest_mops = mtable
                    .0
                    .iter()
                    .fold(0, |acc, entry| acc + (!entry.entry.atype.is_init() as u64));

                self.assign_fixed(&mut ctx)?;
                ctx.reset();

                let (rest_memory_finalize_ops, _) = mtable.count_rest_memory_finalize_ops();

                #[cfg(feature = "continuation")]
                let rest_memory_finalize_ops_cell =
                    self.constrain_rest_memory_finalize_ops(&mut ctx, rest_memory_finalize_ops)?;

                let rest_mops_cell = self.constrain_rest_mops_permutation(&mut ctx, rest_mops)?;

                self.assign_entries(region, mtable, rest_mops, rest_memory_finalize_ops)?;

                cfg_if::cfg_if! {
                    if #[cfg(feature="continuation")] {
                        Ok((rest_mops_cell, Some(rest_memory_finalize_ops_cell)))
                    } else {
                        // Useless rest_memory_finalize_ops if continuation is disabled
                        Ok((rest_mops_cell, None,))
                    }
                }
            },
        )
    }
}
