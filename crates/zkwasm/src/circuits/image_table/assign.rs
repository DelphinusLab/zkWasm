use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
use specs::state::InitializationState;

use super::ImageTableChip;
use super::ImageTableLayouter;
use crate::circuits::image_table::INIT_MEMORY_ENTRIES_OFFSET;
use crate::circuits::utils::Context;

impl<F: FieldExt> ImageTableChip<F> {
    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<ImageTableLayouter<AssignedCell<F, F>>, Error> {
        macro_rules! assign {
            ($ctx:expr, $col:expr, $v:expr) => {{
                let cell =
                    $ctx.region
                        .assign_advice(|| "image table", $col, $ctx.offset, || Ok($v))?;

                $ctx.next();

                Ok::<AssignedCell<F, F>, Error>(cell)
            }};
        }

        fn assign_and_perm_initialization_state<F: FieldExt>(
            ctx: &mut Context<F>,
            col: Column<Advice>,
            initialization_state: &InitializationState<AssignedCell<F, F>>,
        ) -> Result<InitializationState<AssignedCell<F, F>>, Error> {
            let initialization_state = initialization_state.map(|field| {
                field
                    .copy_advice(
                        || "image table: initialization state",
                        &mut ctx.region,
                        col,
                        ctx.offset,
                    )
                    .unwrap();

                ctx.next();

                field.clone()
            });

            Ok::<_, Error>(initialization_state)
        }

        fn assign_static_frame_entries<F: FieldExt>(
            ctx: &mut Context<F>,
            col: Column<Advice>,
            static_frame_entries: &Vec<(AssignedCell<F, F>, AssignedCell<F, F>)>,
        ) -> Result<(), Error> {
            for (enable, entry) in static_frame_entries {
                enable.copy_advice(
                    || "image table: static frame entry",
                    &mut ctx.region,
                    col,
                    ctx.offset,
                )?;
                ctx.next();

                entry.copy_advice(
                    || "image table: static frame entry",
                    &mut ctx.region,
                    col,
                    ctx.offset,
                )?;
                ctx.next();
            }

            Ok(())
        }

        fn assign_instructions<F: FieldExt>(
            ctx: &mut Context<F>,
            col: Column<Advice>,
            instructions: &Vec<F>,
        ) -> Result<Vec<AssignedCell<F, F>>, Error> {
            let entries = instructions
                .iter()
                .map(|entry| assign!(ctx, col, *entry))
                .collect::<Result<Vec<_>, Error>>()?;

            Ok(entries)
        }

        fn assign_br_table<F: FieldExt>(
            ctx: &mut Context<F>,
            col: Column<Advice>,
            br_table: &Vec<F>,
        ) -> Result<Vec<AssignedCell<F, F>>, Error> {
            let entries = br_table
                .iter()
                .map(|entry| assign!(ctx, col, *entry))
                .collect::<Result<Vec<_>, Error>>()?;

            Ok(entries)
        }

        fn assign_init_memory_entries<F: FieldExt>(
            ctx: &mut Context<F>,
            col: Column<Advice>,
            init_memory_entries: &Vec<F>,
        ) -> Result<(), Error> {
            assert!(ctx.offset < INIT_MEMORY_ENTRIES_OFFSET);
            ctx.offset = INIT_MEMORY_ENTRIES_OFFSET;

            for entry in init_memory_entries {
                assign!(ctx, col, *entry)?;
            }

            Ok(())
        }

        layouter.assign_region(
            || "pre image table",
            |region| {
                let mut ctx = Context::new(region);

                let initialization_state = assign_and_perm_initialization_state(
                    &mut ctx,
                    self.config.col,
                    &permutation_cells.initialization_state,
                )?;
                assign_static_frame_entries(
                    &mut ctx,
                    self.config.col,
                    &permutation_cells.static_frame_entries,
                )?;
                let instructions = assign_instructions(
                    &mut ctx,
                    self.config.col,
                    image_table.instructions.as_ref().unwrap(),
                )
                .ok();
                let br_table = assign_br_table(
                    &mut ctx,
                    self.config.col,
                    image_table.br_table.as_ref().unwrap(),
                )
                .ok();
                assign_init_memory_entries(
                    &mut ctx,
                    self.config.col,
                    &image_table.init_memory_entries.as_ref().unwrap(),
                )?;

                Ok(ImageTableLayouter {
                    initialization_state,
                    static_frame_entries: permutation_cells.static_frame_entries.clone(),
                    instructions,
                    br_table,
                    init_memory_entries: None,
                    rest_memory_writing_ops: None,
                })
            },
        )
    }
}
