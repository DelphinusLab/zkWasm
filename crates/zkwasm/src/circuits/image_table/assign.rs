use std::cell::RefCell;
use std::rc::Rc;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;

use super::ImageTableChip;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;
use crate::circuits::utils::Context;

impl<F: FieldExt> ImageTableChip<F> {
    pub(crate) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table_assigner: &mut ImageTableAssigner,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<ImageTableLayouter<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "pre image table",
            |region| {
                let ctx = Rc::new(RefCell::new(Context::new(region)));

                macro_rules! assign {
                    ($v:expr) => {{
                        let offset = ctx.borrow().offset;

                        let cell = ctx.borrow_mut().region.assign_advice(
                            || "pre image table",
                            self.config.col,
                            offset,
                            || Ok($v),
                        );

                        ctx.borrow_mut().next();

                        cell
                    }};
                }

                let initialization_state_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let initialization_state =
                        permutation_cells.initialization_state.map(|field| {
                            let offset = ctx.borrow().offset;

                            field.copy_advice(
                                || "image table: initialization state",
                                &mut ctx.borrow_mut().region,
                                self.config.col,
                                offset,
                            )?;

                            ctx.borrow_mut().next();

                            Ok(field.clone())
                        });

                    initialization_state.transpose()
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

                            Ok((enable.clone(), entry.clone()))
                        })
                        .collect::<Result<Vec<_>, Error>>()
                };

                let instruction_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .instructions
                        .iter()
                        .map(|entry| assign!(*entry))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .br_table_entires
                        .iter()
                        .map(|entry| assign!(*entry))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, end_offset| {
                    ctx.borrow_mut().offset = start_offset;

                    (start_offset..end_offset)
                        .map(|_| assign!(F::zero()))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let init_memory_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    image_table
                        .init_memory_entries
                        .iter()
                        .map(|entry| {

                            assign!(*entry)

                        })
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
                    instructions: result.instructions,
                    br_table_entires: result.br_table_entires,
                    padding_entires: result.padding_entires,
                    init_memory_entries: result.init_memory_entries,
                })
            },
        )
    }
}
