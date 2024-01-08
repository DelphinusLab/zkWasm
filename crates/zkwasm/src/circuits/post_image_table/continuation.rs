use std::cell::RefCell;
use std::collections::HashSet;
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
use specs::encode::init_memory_table::MEMORY_ADDRESS_OFFSET;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use specs::mtable::LocationType;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::image_table::image_table_offset_to_memory_location;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;
use crate::circuits::utils::image_table::GLOBAL_CAPABILITY;
use crate::circuits::utils::image_table::STACK_CAPABILITY;
use crate::circuits::utils::Context;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::next;

pub const POST_IMAGE_TABLE: &str = "post_img_col";

#[derive(Clone)]
pub(in crate::circuits) struct PostImageTableConfig<F: FieldExt> {
    memory_addr_sel: Column<Fixed>,
    post_image_table: Column<Advice>,
    update: Column<Advice>,
    rest_memory_finalized_count: Column<Advice>,
    memory_finalized_lookup_encode: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableConfig<F> {
    pub(in crate::circuits) fn configure(
        meta: &mut ConstraintSystem<F>,
        memory_addr_sel: Option<Column<Fixed>>,
        memory_table: &MemoryTableConfig<F>,
        pre_image_table: &ImageTableConfig<F>,
    ) -> Self {
        let memory_addr_sel = memory_addr_sel.unwrap();
        let update = meta.advice_column();
        let rest_memory_finalized_count = meta.advice_column();
        let post_image_table = meta.named_advice_column(POST_IMAGE_TABLE.to_owned());
        let memory_finalized_lookup_encode = meta.advice_column();

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

        meta.create_gate("post image table: memory_finalized_lookup_encode", |meta| {
            vec![
                fixed_curr!(meta, memory_addr_sel)
                    * curr!(meta, update)
                    * (fixed_curr!(meta, memory_addr_sel) * constant_from!(MEMORY_ADDRESS_OFFSET)
                        + curr!(meta, post_image_table)
                        - curr!(meta, memory_finalized_lookup_encode)),
            ]
        });

        memory_table.configure_in_post_init_memory_table(
            meta,
            "post image table: lookup updating value",
            |meta| curr!(meta, memory_finalized_lookup_encode),
        );

        Self {
            memory_addr_sel,
            post_image_table,
            update,
            rest_memory_finalized_count,
            memory_finalized_lookup_encode,
            _mark: PhantomData,
        }
    }
}

pub(in crate::circuits) struct PostImageTableChip<F: FieldExt> {
    config: PostImageTableConfig<F>,
}

impl<F: FieldExt> PostImageTableChip<F> {
    pub(in crate::circuits) fn new(config: PostImageTableConfig<F>) -> Self {
        Self { config }
    }

    pub(in crate::circuits) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table_assigner: &mut ImageTableAssigner,
        post_image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
        rest_memory_writing_ops_cell: Option<AssignedCell<F, F>>,
        rest_memory_writing_ops: F,
        memory_finalized_set: HashSet<(LocationType, u32)>,
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

                            field.copy_advice(
                                || "image table: initialization state",
                                &mut ctx.borrow_mut().region,
                                self.config.post_image_table,
                                offset,
                            )?;

                            ctx.borrow_mut().next();

                            Ok(field.clone())
                        });

                    initialization_state.transpose()
                };

                let static_frame_entries_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let mut cells = vec![];

                    for (enable, entry) in &permutation_cells.static_frame_entries {
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

                        cells.push((enable.clone(), entry.clone()));
                    }

                    Ok(cells.try_into().expect(&format!(
                        "The number of static frame entries should be {}",
                        STATIC_FRAME_ENTRY_NUMBER
                    )))
                };

                let instruction_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    permutation_cells
                        .instructions
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
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    permutation_cells
                        .br_table_entires
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
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, _| {
                    ctx.borrow_mut().offset = start_offset;

                    permutation_cells
                        .padding_entires
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

                        for i in 0..STACK_CAPABILITY {
                            assign_address!(LocationType::Stack, i)?;
                        }

                        for i in 0..GLOBAL_CAPABILITY {
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

                        rest_memory_writing_ops_cell.as_ref().unwrap().copy_advice(
                            || "post image table: init memory",
                            &mut ctx.borrow_mut().region,
                            self.config.rest_memory_finalized_count,
                            offset,
                        )?;
                    }

                    let entries = {
                        let mut offset = base_offset;

                        let mut rest_memory_writing_ops = rest_memory_writing_ops;

                        post_image_table
                            .init_memory_entries
                            .iter()
                            .map(|post| {
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

                                let position = image_table_offset_to_memory_location(offset);

                                if memory_finalized_set.contains(&position) {
                                    ctx.borrow_mut().region.assign_advice(
                                        || "post image table: init memory",
                                        self.config.update,
                                        offset,
                                        || Ok(F::one()),
                                    )?;

                                    let address: BigUint =
                                        encode_init_memory_table_address::<BigUint>(
                                            (position.0 as u64).into(),
                                            position.1.into(),
                                        ) * MEMORY_ADDRESS_OFFSET;

                                    ctx.borrow_mut().region.assign_advice(
                                        || "post image table: init memory lookup",
                                        self.config.memory_finalized_lookup_encode,
                                        offset,
                                        || Ok(bn_to_field::<F>(&address) + *post),
                                    )?;

                                    rest_memory_writing_ops = rest_memory_writing_ops - F::one();
                                }

                                offset += 1;

                                Ok(entry)
                            })
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
