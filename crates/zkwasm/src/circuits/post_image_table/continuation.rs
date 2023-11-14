use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::Region;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use specs::encode::init_memory_table::encode_init_memory_table_address;
use specs::mtable::LocationType;
use specs::state::InitializationState;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::image_table::ImageTableLayouter;
use crate::circuits::image_table::INIT_MEMORY_ENTRIES_OFFSET;
use crate::circuits::image_table::PAGE_ENTRIES;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::utils::bn_to_field;
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
    circuit_maximal_pages: u32,
}

impl<F: FieldExt> PostImageTableChipTrait<F, ContinuationPostImageTableConfig<F>>
    for ContinuationPostImageTableChip<F>
{
    fn new(config: ContinuationPostImageTableConfig<F>, circuit_maximal_pages: u32) -> Self {
        Self {
            config,
            circuit_maximal_pages,
        }
    }

    fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        pre_image_table: ImageTableLayouter<F>,
        post_image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<(), Error> {
        fn init_sel<F: FieldExt>(
            region: &mut Region<F>,
            sel: Column<Fixed>,
            rest_memory_finalized_count: Column<Advice>,
            circuit_maximal_pages: u32,
        ) -> Result<(), Error> {
            let mut offset = INIT_MEMORY_ENTRIES_OFFSET;

            region.assign_fixed(|| "post image table: init", sel, offset, || Ok(F::zero()))?;

            offset += 1;

            macro_rules! assign_address {
                ($l:expr, $o:expr) => {{
                    region.assign_fixed(
                        || "post image table: init",
                        sel,
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

            for i in 0..(circuit_maximal_pages * PAGE_ENTRIES) {
                assign_address!(LocationType::Heap, i)?;
            }

            region.assign_advice_from_constant(
                || "post image table: init memory",
                rest_memory_finalized_count,
                offset,
                F::zero(),
            )?;

            Ok(())
        }

        fn assign_and_perm_initialization_state<F: FieldExt>(
            ctx: &mut Context<F>,
            post_image_table_col: Column<Advice>,
            initialization_state: &InitializationState<AssignedCell<F, F>>,
        ) -> Result<(), Error> {
            Ok::<_, Error>(initialization_state.for_each(|field| {
                field
                    .copy_advice(
                        || "image table: initialization state",
                        &mut ctx.region,
                        post_image_table_col,
                        ctx.offset,
                    )
                    .unwrap();

                ctx.next();
            }))
        }

        fn assign_static_frame_entries<F: FieldExt>(
            ctx: &mut Context<F>,
            post_image_table_col: Column<Advice>,
            static_frame_entries: &Vec<(AssignedCell<F, F>, AssignedCell<F, F>)>,
        ) -> Result<(), Error> {
            for (enable, entry) in static_frame_entries {
                enable.copy_advice(
                    || "image table: static frame entry",
                    &mut ctx.region,
                    post_image_table_col,
                    ctx.offset,
                )?;
                ctx.next();

                entry.copy_advice(
                    || "image table: static frame entry",
                    &mut ctx.region,
                    post_image_table_col,
                    ctx.offset,
                )?;
                ctx.next();
            }

            Ok(())
        }

        fn assign_instructions<F: FieldExt>(
            ctx: &mut Context<F>,
            post_image_table_col: Column<Advice>,
            instructions: &Vec<AssignedCell<F, F>>,
        ) -> Result<(), Error> {
            for cell in instructions {
                cell.copy_advice(
                    || "post image table: instructions",
                    &mut ctx.region,
                    post_image_table_col,
                    ctx.offset,
                )?;

                ctx.next();
            }

            Ok(())
        }

        fn assign_br_table<F: FieldExt>(
            ctx: &mut Context<F>,
            post_image_table_col: Column<Advice>,
            br_table: &Vec<AssignedCell<F, F>>,
        ) -> Result<(), Error> {
            for cell in br_table {
                cell.copy_advice(
                    || "post image table: instructions",
                    &mut ctx.region,
                    post_image_table_col,
                    ctx.offset,
                )?;

                ctx.next();
            }

            Ok(())
        }

        fn assign_init_memory_entries<F: FieldExt>(
            ctx: &mut Context<F>,
            sel: Column<Fixed>,
            post_image_table_col: Column<Advice>,
            update_col: Column<Advice>,
            rest_memory_finalized_ops_col: Column<Advice>,
            pre_image_table: &ImageTableLayouter<F>,
            post_image_table: &ImageTableLayouter<F>,
            permutation_cells: &ImageTableLayouter<AssignedCell<F, F>>,
            circuit_maximal_pages: u32,
        ) -> Result<(), Error> {
            assert!(ctx.offset < INIT_MEMORY_ENTRIES_OFFSET);
            ctx.offset = INIT_MEMORY_ENTRIES_OFFSET;

            assert_eq!(
                pre_image_table.init_memory_entries.as_ref().unwrap().len(),
                post_image_table.init_memory_entries.as_ref().unwrap().len()
            );

            init_sel(
                &mut ctx.region,
                sel,
                rest_memory_finalized_ops_col,
                circuit_maximal_pages,
            )?;

            permutation_cells
                .rest_memory_writing_ops
                .as_ref()
                .unwrap()
                .copy_advice(
                    || "post image table: init memory",
                    &mut ctx.region,
                    rest_memory_finalized_ops_col,
                    ctx.offset,
                )?;

            let mut rest_memory_writing_ops = *permutation_cells
                .rest_memory_writing_ops
                .as_ref()
                .unwrap()
                .value()
                .unwrap();

            for (pre, post) in pre_image_table
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
            {
                ctx.region.assign_advice(
                    || "post image table: init memory",
                    post_image_table_col,
                    ctx.offset,
                    || Ok(*post),
                )?;

                ctx.region.assign_advice(
                    || "post image table: init memory",
                    rest_memory_finalized_ops_col,
                    ctx.offset,
                    || Ok(rest_memory_writing_ops),
                )?;

                if pre != post {
                    ctx.region.assign_advice(
                        || "post image table: init memory",
                        update_col,
                        ctx.offset,
                        || Ok(F::one()),
                    )?;

                    rest_memory_writing_ops = rest_memory_writing_ops - F::one();
                }

                ctx.next();
            }

            assert_eq!(rest_memory_writing_ops, F::zero());

            Ok::<_, Error>(())
        }

        layouter.assign_region(
            || "post image table",
            |region| {
                let mut ctx = Context::new(region);

                assign_and_perm_initialization_state(
                    &mut ctx,
                    self.config.post_image_table,
                    &permutation_cells.initialization_state,
                )?;
                assign_static_frame_entries(
                    &mut ctx,
                    self.config.post_image_table,
                    &permutation_cells.static_frame_entries,
                )?;
                assign_instructions(
                    &mut ctx,
                    self.config.post_image_table,
                    permutation_cells.instructions.as_ref().unwrap(),
                )?;
                assign_br_table(
                    &mut ctx,
                    self.config.post_image_table,
                    permutation_cells.br_table.as_ref().unwrap(),
                )?;
                assign_init_memory_entries(
                    &mut ctx,
                    self.config.memory_addr_sel,
                    self.config.post_image_table,
                    self.config.update,
                    self.config.rest_memory_finalized_count,
                    &pre_image_table,
                    &post_image_table,
                    &permutation_cells,
                    self.circuit_maximal_pages,
                )?;

                Ok(())
            },
        )
    }
}
