use std::cell::RefCell;
use std::rc::Rc;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use super::ImageTableChip;
use super::ImageTableLayouter;
use crate::circuits::image_table::INIT_MEMORY_ENTRIES_OFFSET;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::Context;

impl<F: FieldExt> ImageTableChip<F> {
    pub(crate) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table_assigner: &mut ImageTableAssigner<
            INIT_MEMORY_ENTRIES_OFFSET,
            DEFAULT_VALUE_STACK_LIMIT,
            DEFAULT_VALUE_STACK_LIMIT,
        >,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<ImageTableLayouter<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "pre image table",
            |region| {
                let ctx = Rc::new(RefCell::new(Context::new(region)));

                let initialization_state_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let initialization_state =
                        permutation_cells.initialization_state.map(|field| {
                            let offset = ctx.borrow().offset;

                            field
                                .copy_advice(
                                    || "image table: initialization state",
                                    &mut ctx.borrow_mut().region,
                                    self.config.col,
                                    offset,
                                )
                                .unwrap();

                            ctx.borrow_mut().next();

                            field.clone()
                        });

                    Ok::<_, Error>(initialization_state)
                };

                let static_frame_entries_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    permutation_cells
                        .static_frame_entries
                        .iter()
                        .map(|(enable, entry)| {
                            let offset = ctx.borrow().offset;

                            enable.copy_advice(
                                || "image table: static frame entry",
                                &mut ctx.borrow_mut().region,
                                self.config.col,
                                offset,
                            )?;
                            ctx.borrow_mut().next();

                            let offset = ctx.borrow().offset;

                            entry.copy_advice(
                                || "image table: static frame entry",
                                &mut ctx.borrow_mut().region,
                                self.config.col,
                                offset,
                            )?;
                            ctx.borrow_mut().next();

                            Ok::<_, Error>((enable.clone(), entry.clone()))
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let instruction_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .instructions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;

                            let cell = ctx.borrow_mut().region.assign_advice(
                                || "image table",
                                self.config.col,
                                offset,
                                || Ok(*entry),
                            );

                            ctx.borrow_mut().next();

                            cell
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .br_table
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;

                            let cell = ctx.borrow_mut().region.assign_advice(
                                || "image table",
                                self.config.col,
                                offset,
                                || Ok(*entry),
                            );

                            ctx.borrow_mut().next();

                            cell
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, end_offset| {
                    (start_offset..end_offset)
                        .map(|offset| {
                            ctx.borrow_mut().region.assign_advice(
                                || "image table: padding",
                                self.config.col,
                                offset,
                                || Ok(F::zero()),
                            )
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let init_memory_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .init_memory_entries
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;
                            let cell = ctx.borrow_mut().region.assign_advice(
                                || "image table",
                                self.config.col,
                                offset,
                                || Ok(*entry),
                            );

                            ctx.borrow_mut().next();

                            cell
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let result = image_table_assigner.exec(
                    initialization_state_handler,
                    static_frame_entries_handler,
                    instruction_handler,
                    br_table_handler,
                    padding_handler,
                    init_memory_handler,
                )?;

                Ok(ImageTableLayouter {
                    initialization_state: result.initialization_state,
                    static_frame_entries: result.static_frame_entries,
                    instructions: Some(result.instructions),
                    br_table: Some(result.br_table_entires),
                    padding: Some(result.padding_entires),
                    init_memory_entries: None,
                    rest_memory_writing_ops: None,
                })
            },
        )
    }
}
