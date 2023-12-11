use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use specs::encode::init_memory_table::encode_init_memory_table_address;
use specs::mtable::LocationType;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::image_table::ImageTableLayouter;
use crate::circuits::image_table::INIT_MEMORY_ENTRIES_OFFSET;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::Context;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::next;

use super::PostImageTableChipTrait;
use super::PostImageTableConfigTrait;

#[derive(Clone)]
pub(in crate::circuits) struct ContinuationPostImageTableConfig<F: FieldExt> {
    memory_addr_sel: Column<Fixed>,
    post_image_table: Column<Advice>,
    update: Column<Advice>,
    rest_memory_finalized_count: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableConfigTrait<F> for ContinuationPostImageTableConfig<F> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        memory_addr_sel: Column<Fixed>,
        memory_table: &MemoryTableConfig<F>,
        pre_image_table: &ImageTableConfig<F>,
    ) -> Self {
        let update = meta.advice_column();
        let rest_memory_finalized_count = meta.advice_column();
        let post_image_table = meta.advice_column();

        meta.enable_equality(rest_memory_finalized_count);
        meta.enable_equality(post_image_table);

        meta.create_gate("post image table: update", |meta| {
            vec![
                // Update is a bit.
                fixed_curr!(meta, memory_addr_sel)
                    * curr!(meta, update)
                    * (curr!(meta, update) - constant_from!(1)),
                // count -= 1 iff update = 1.
                fixed_curr!(meta, memory_addr_sel)
                    * (curr!(meta, rest_memory_finalized_count)
                        - next!(meta, rest_memory_finalized_count)
                        - curr!(meta, update)),
                // If update is 0, value should keep the same.
                fixed_curr!(meta, memory_addr_sel)
                    * (constant_from!(1) - curr!(meta, update))
                    * (curr!(meta, post_image_table) - pre_image_table.expr(meta)),
            ]
        });

        memory_table.configure_in_post_init_memory_table(
            meta,
            "post image table: lookup updating value",
            |meta| {
                (
                    fixed_curr!(meta, memory_addr_sel) * curr!(meta, update),
                    curr!(meta, post_image_table) * curr!(meta, update),
                )
            },
        );

        Self {
            memory_addr_sel,
            post_image_table,
            update,
            rest_memory_finalized_count,
            _mark: PhantomData,
        }
    }
}

pub(in crate::circuits) struct ContinuationPostImageTableChip<F: FieldExt> {
    config: ContinuationPostImageTableConfig<F>,
}

impl<F: FieldExt> PostImageTableChipTrait<F, ContinuationPostImageTableConfig<F>>
    for ContinuationPostImageTableChip<F>
{
    fn new(config: ContinuationPostImageTableConfig<F>) -> Self {
        Self { config }
    }

    fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table_assigner: &mut ImageTableAssigner<
            INIT_MEMORY_ENTRIES_OFFSET,
            DEFAULT_VALUE_STACK_LIMIT,
            DEFAULT_VALUE_STACK_LIMIT,
        >,
        pre_image_table: ImageTableLayouter<F>,
        post_image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
        rest_memory_writing_ops: F,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "post image table",
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
                                    self.config.post_image_table,
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
                                self.config.post_image_table,
                                offset,
                            )?;
                            ctx.borrow_mut().next();

                            let offset = ctx.borrow().offset;

                            entry.copy_advice(
                                || "image table: static frame entry",
                                &mut ctx.borrow_mut().region,
                                self.config.post_image_table,
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

                    permutation_cells
                        .instructions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;

                            let entry = entry.copy_advice(
                                || "image table",
                                &mut ctx.borrow_mut().region,
                                self.config.post_image_table,
                                offset,
                            )?;

                            ctx.borrow_mut().next();

                            Ok(entry)
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    permutation_cells
                        .br_table
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;

                            let entry = entry.copy_advice(
                                || "image table",
                                &mut ctx.borrow_mut().region,
                                self.config.post_image_table,
                                offset,
                            )?;

                            ctx.borrow_mut().next();

                            Ok(entry)
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, _| {
                    ctx.borrow_mut().offset = start_offset;

                    permutation_cells
                        .padding
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let offset = ctx.borrow().offset;

                            let entry = entry.copy_advice(
                                || "image table",
                                &mut ctx.borrow_mut().region,
                                self.config.post_image_table,
                                offset,
                            )?;

                            ctx.borrow_mut().next();

                            Ok(entry)
                        })
                        .collect::<Vec<Result<_, Error>>>()
                        .into_iter()
                        .collect::<Result<Vec<_>, Error>>()
                };

                let heap_capability = image_table_assigner.heap_capability;
                let init_memory_entries_handler = |base_offset| {
                    // Assign fixed and constant
                    {
                        let mut offset = base_offset;

                        ctx.borrow_mut().region.assign_fixed(
                            || "post image table",
                            self.config.memory_addr_sel,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        offset += 1;

                        macro_rules! assign_address {
                            ($l:expr, $o:expr) => {{
                                ctx.borrow_mut().region.assign_fixed(
                                    || "post image table: init",
                                    self.config.memory_addr_sel,
                                    offset,
                                    || {
                                        Ok(bn_to_field(&encode_init_memory_table_address(
                                            BigUint::from($l as u64),
                                            BigUint::from($o as u64),
                                        )))
                                    },
                                )?;

                                offset += 1;

                                Ok::<_, Error>(())
                            }};
                        }

                        for i in 0..DEFAULT_VALUE_STACK_LIMIT {
                            assign_address!(LocationType::Stack, i)?;
                        }

                        for i in 0..DEFAULT_VALUE_STACK_LIMIT {
                            assign_address!(LocationType::Global, i)?;
                        }

                        for i in 0..heap_capability {
                            assign_address!(LocationType::Heap, i)?;
                        }

                        ctx.borrow_mut().region.assign_advice_from_constant(
                            || "post image table: init memory",
                            self.config.rest_memory_finalized_count,
                            offset,
                            F::zero(),
                        )?;
                    }

                    {
                        // First line is placeholder for default lookup
                        let offset = base_offset + 1;

                        permutation_cells
                            .rest_memory_writing_ops
                            .as_ref()
                            .unwrap()
                            .copy_advice(
                                || "post image table: init memory",
                                &mut ctx.borrow_mut().region,
                                self.config.rest_memory_finalized_count,
                                offset,
                            )?;
                    }

                    let entries = {
                        let mut offset = base_offset;

                        let mut rest_memory_writing_ops = rest_memory_writing_ops;

                        pre_image_table
                            .init_memory_entries
                            .as_ref()
                            .unwrap()
                            .iter()
                            .zip(
                                post_image_table
                                    .init_memory_entries
                                    .as_ref()
                                    .unwrap()
                                    .iter(),
                            )
                            .map(|(pre, post)| {
                                let entry = ctx.borrow_mut().region.assign_advice(
                                    || "post image table: init memory",
                                    self.config.post_image_table,
                                    offset,
                                    || Ok(*post),
                                )?;

                                ctx.borrow_mut().region.assign_advice(
                                    || "post image table: init memory",
                                    self.config.rest_memory_finalized_count,
                                    offset,
                                    || Ok(rest_memory_writing_ops),
                                )?;

                                if pre != post {
                                    ctx.borrow_mut().region.assign_advice(
                                        || "post image table: init memory",
                                        self.config.update,
                                        offset,
                                        || Ok(F::one()),
                                    )?;

                                    rest_memory_writing_ops = rest_memory_writing_ops - F::one();
                                }

                                offset += 1;

                                Ok(entry)
                            })
                            .collect::<Vec<Result<_, Error>>>()
                            .into_iter()
                            .collect::<Result<Vec<_>, Error>>()
                    }?;

                    Ok(entries)
                };

                image_table_assigner.exec(
                    initialization_state_handler,
                    static_frame_entries_handler,
                    instruction_handler,
                    br_table_handler,
                    padding_handler,
                    init_memory_entries_handler,
                )?;

                Ok(())
            },
        )
    }
}
