use std::cell::RefCell;
use std::rc::Rc;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;

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
    ) -> Result<ImageTableLayouter<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "pre image table",
            |region| {
                let ctx = Rc::new(RefCell::new(Context::new(region)));

                cfg_if::cfg_if! {
                    if #[cfg(feature="uniform-circuit")] {
                        macro_rules! assign {
                            ($v: expr) => {{
                                let offset = ctx.borrow().offset;

                                let cell = ctx
                                    .borrow_mut()
                                    .region
                                    .assign_advice(
                                        || "pre image table",
                                        self.config.col,
                                        offset,
                                        || Ok($v),
                                    )?;

                                ctx.borrow_mut().next();

                                Ok::<_, Error>(cell)
                            }};
                        }
                    } else {
                        macro_rules! assign {
                            ($v: expr) => {{
                                let offset = ctx.borrow().offset;

                                let cell = ctx
                                    .borrow_mut()
                                    .region
                                    .assign_fixed(
                                        || "pre image table",
                                        self.config.col,
                                        offset,
                                        || Ok($v),
                                    )?;

                                ctx.borrow_mut().next();

                                Ok::<_, Error>(cell)
                            }};
                        }
                    }
                }

                let initialization_state_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let initialization_state = image_table
                        .initialization_state
                        .map(|field| assign!(*field));

                    initialization_state.transpose()
                };

                let static_frame_entries_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let mut cells = vec![];

                    for (enable, entry) in &image_table.static_frame_entries {
                        let enable = assign!(*enable)?;
                        let entry = assign!(*entry)?;

                        cells.push((enable, entry));
                    }

                    Ok(cells.try_into().expect(&format!(
                        "The number of static frame entries should be {}",
                        STATIC_FRAME_ENTRY_NUMBER
                    )))
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
                        .map(|entry| assign!(*entry))
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
