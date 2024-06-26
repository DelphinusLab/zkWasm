use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelSlice;
use rayon::slice::ParallelSliceMut;
use specs::jtable::INHERITED_FRAME_TABLE_ENTRIES;

use super::ImageTableChip;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;
use crate::circuits::utils::Context;

cfg_if::cfg_if! {
    if #[cfg(feature="uniform-circuit")] {
        macro_rules! assign_option {
            ($ctx:expr, $col: expr, $v: expr) => {{
                let cell = $ctx
                    .region
                    .assign_advice(
                        || "pre image table",
                        $col,
                        $ctx.offset,
                        || $v,
                    ).unwrap();

                $ctx.next();

                Ok::<_, Error>(cell)
            }};
        }
    } else {
        macro_rules! assign_option {
            ($ctx:expr, $col: expr, $v: expr) => {{
                let cell = $ctx
                    .region
                    .assign_fixed(
                        || "pre image table",
                        $col,
                        $ctx.offset,
                        || $v,
                    ).unwrap();

                $ctx.next();

                Ok::<_, Error>(cell)
            }};
        }
    }
}

macro_rules! assign {
    ($ctx:expr, $col: expr, $v: expr) => {{
        assign_option!($ctx, $col, Ok($v))
    }};
}

impl<F: FieldExt> ImageTableChip<F> {
    pub(crate) fn assign(
        &self,
        layouter: impl Layouter<F>,
        image_table_assigner: &ImageTableAssigner,
        image_table: ImageTableLayouter<F>,
    ) -> Result<ImageTableLayouter<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "pre image table",
            |region| {
                let initialization_state_handler = |base_offset| {
                    let mut ctx = Context::new(region);
                    ctx.offset = base_offset;

                    let initialization_state = image_table
                        .initialization_state
                        .map(|field: &F| assign!(ctx, self.config.col, *field));

                    initialization_state.transpose()
                };

                let inherited_frame_entries_handler = |base_offset| {
                    let mut ctx = Context::new(region);
                    ctx.offset = base_offset;

                    let mut cells = Vec::with_capacity(INHERITED_FRAME_TABLE_ENTRIES);

                    for entry in image_table.inherited_frame_entries.iter() {
                        let entry = assign!(ctx, self.config.col, *entry)?;

                        cells.push(entry);
                    }

                    Ok(cells.try_into().unwrap())
                };

                let instruction_handler = |base_offset| {
                    let mut ctx = Context::new(region);
                    ctx.offset = base_offset;

                    image_table
                        .instructions
                        .iter()
                        .map(|entry| assign!(ctx, self.config.col, *entry))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    let mut ctx = Context::new(region);
                    ctx.offset = base_offset;

                    image_table
                        .br_table_entires
                        .iter()
                        .map(|entry| assign!(ctx, self.config.col, *entry))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, end_offset| {
                    let mut ctx = Context::new(region);
                    ctx.offset = start_offset;

                    (start_offset..end_offset)
                        .map(|_| assign!(ctx, self.config.col, F::zero()))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let init_memory_handler = |base_offset| {
                    const THREAD: usize = 4;
                    let chunk_size = if image_table.init_memory_entries.is_empty() {
                        1
                    } else {
                        (image_table.init_memory_entries.len() + THREAD - 1) / THREAD
                    };

                    let mut cells = Vec::with_capacity(image_table.init_memory_entries.len());
                    let remaining = cells.spare_capacity_mut();

                    image_table
                        .init_memory_entries
                        .par_chunks(chunk_size)
                        .zip(remaining.par_chunks_mut(chunk_size))
                        .enumerate()
                        .for_each(|(chunk_index, (entries, cells))| {
                            let mut ctx = Context::new(region);
                            // start from 'base_offset" because 'encode_compilation_table_values' have inserted an empty at the beginning.
                            ctx.offset = base_offset + chunk_index * chunk_size;

                            entries.iter().enumerate().for_each(|(index, entry)| {
                                let cell = assign!(ctx, self.config.col, *entry).unwrap();

                                cells[index].write(cell);
                            });
                        });

                    unsafe {
                        cells.set_len(image_table.init_memory_entries.len());
                    }

                    Ok(cells)
                };

                let result = image_table_assigner.exec(
                    initialization_state_handler,
                    inherited_frame_entries_handler,
                    instruction_handler,
                    br_table_handler,
                    padding_handler,
                    init_memory_handler,
                )?;

                Ok(ImageTableLayouter {
                    initialization_state: result.initialization_state,
                    inherited_frame_entries: result.inherited_frame_entries,
                    instructions: result.instructions,
                    br_table_entires: result.br_table_entires,
                    padding_entires: result.padding_entires,
                    init_memory_entries: result.init_memory_entries,
                })
            },
        )
    }
}
