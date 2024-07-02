use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use specs::encode::init_memory_table::encode_init_memory_table_address;
use specs::encode::init_memory_table::MEMORY_ADDRESS_OFFSET;
use specs::mtable::LocationType;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::jtable::JumpTableConfig;
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

cfg_if::cfg_if! {
    if #[cfg(feature="uniform-circuit")] {
        macro_rules! assign_option {
            ($ctx:expr, $col: expr, $v: expr) => {{
                let offset = $ctx.borrow().offset;

                let cell = $ctx
                    .borrow_mut()
                    .region
                    .assign_advice(
                        || "pre image table",
                        $col,
                        offset,
                        || $v,
                    )?;

                $ctx.borrow_mut().next();

                Ok::<_, Error>(cell)
            }};
        }
    } else {
        macro_rules! assign_option {
            ($ctx:expr, $col: expr, $v: expr) => {{
                let offset = $ctx.borrow().offset;

                let cell = $ctx
                    .borrow_mut()
                    .region
                    .assign_fixed(
                        || "pre image table",
                        $col,
                        offset,
                        || $v,
                    )?;

                $ctx.borrow_mut().next();

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

#[derive(Clone)]
pub(in crate::circuits) struct PostImageTableConfig<F: FieldExt> {
    memory_addr_sel: Column<Fixed>,
    inherited_frame_table_sel: Column<Fixed>,
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
        frame_table: &JumpTableConfig<F>,
        pre_image_table: &ImageTableConfig<F>,
    ) -> Self {
        let memory_addr_sel = memory_addr_sel.unwrap();
        let update = meta.advice_column();
        let rest_memory_finalized_count = meta.advice_column();
        let post_image_table = meta.named_advice_column(POST_IMAGE_TABLE.to_owned());
        let memory_finalized_lookup_encode = meta.advice_column();
        let inherited_frame_table_sel = meta.fixed_column();

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

        frame_table.configure_lookup_in_frame_table(
            meta,
            "post image table: extract unreturned frame table entries",
            |meta| {
                (
                    fixed_curr!(meta, inherited_frame_table_sel),
                    constant_from!(0),
                    curr!(meta, post_image_table) * fixed_curr!(meta, inherited_frame_table_sel),
                )
            },
        );

        Self {
            memory_addr_sel,
            inherited_frame_table_sel,
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
        layouter: impl Layouter<F>,
        image_table_assigner: &ImageTableAssigner,
        post_image_table: ImageTableLayouter<F>,
        rest_memory_finalized_count: u32,
        memory_finalized_set: HashSet<(LocationType, u32)>,
    ) -> Result<Option<(ImageTableLayouter<AssignedCell<F, F>>, AssignedCell<F, F>)>, Error> {
        layouter.assign_region(
            || "post image table",
            |region| {
                let assigned_rest_memory_finalized_count_cell = Arc::new(Mutex::new(None));

                let ctx = Rc::new(RefCell::new(Context::new(region)));

                let initialization_state_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    let initialization_state = post_image_table
                        .initialization_state
                        .map(|field: &F| Ok(assign!(ctx, self.config.post_image_table, *field)?));

                    initialization_state.transpose()
                };

                let inherited_frame_entries_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;
                    let mut offset = base_offset;

                    let mut cells = vec![];

                    for entry in post_image_table.inherited_frame_entries.iter() {
                        region.assign_fixed(
                            || "post image table: inherited frame table",
                            self.config.inherited_frame_table_sel,
                            offset,
                            || Ok(F::one()),
                        )?;

                        let entry = assign!(ctx, self.config.post_image_table, *entry)?;

                        cells.push(entry);
                        offset += 1;
                    }

                    Ok(cells.try_into().unwrap())
                };

                let instruction_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    post_image_table
                        .instructions
                        .iter()
                        .map(|entry| Ok(assign!(ctx, self.config.post_image_table, *entry)?))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let br_table_handler = |base_offset| {
                    ctx.borrow_mut().offset = base_offset;

                    post_image_table
                        .br_table_entires
                        .iter()
                        .map(|entry| Ok(assign!(ctx, self.config.post_image_table, *entry)?))
                        .collect::<Result<Vec<_>, Error>>()
                };

                let padding_handler = |start_offset, _| {
                    ctx.borrow_mut().offset = start_offset;

                    post_image_table
                        .padding_entires
                        .iter()
                        .map(|entry| Ok(assign!(ctx, self.config.post_image_table, *entry)?))
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
                        ctx.borrow_mut().offset = base_offset + 1;

                        let rest_memory_finalized_count_cell = assign!(
                            ctx,
                            self.config.rest_memory_finalized_count,
                            F::from(rest_memory_finalized_count as u64)
                        )?;

                        *assigned_rest_memory_finalized_count_cell.lock().unwrap() =
                            Some(rest_memory_finalized_count_cell);
                    }

                    let rest_memory_writing_ops = {
                        // start from 'base_offset" instead of 'base_offset + 1' because 'encode_compilation_table_values' have inserted an empty at the beginning.
                        let mut offset = base_offset;

                        let mut rest_memory_writing_ops =
                            F::from(rest_memory_finalized_count as u64);

                        post_image_table
                            .init_memory_entries
                            .iter()
                            .map(|_| {
                                let v = rest_memory_writing_ops;

                                if memory_finalized_set
                                    .contains(&image_table_offset_to_memory_location(offset))
                                {
                                    rest_memory_writing_ops -= F::one();
                                }

                                offset += 1;

                                v
                            })
                            .collect::<Vec<_>>()
                    };

                    let entries = {
                        post_image_table
                            .init_memory_entries
                            .par_iter()
                            .zip(rest_memory_writing_ops.into_par_iter())
                            .enumerate()
                            .map(|(offset, (post, rest_memory_writing_ops))| {
                                // start from 'base_offset" instead of 'base_offset + 1' because 'encode_compilation_table_values' have inserted an empty at the beginning.
                                let offset = base_offset + offset;

                                let entry = region.assign_advice(
                                    || "post image table: init memory",
                                    self.config.post_image_table,
                                    offset,
                                    || Ok(*post),
                                )?;

                                region.assign_advice(
                                    || "post image table: updated memory count",
                                    self.config.rest_memory_finalized_count,
                                    offset,
                                    || Ok(rest_memory_writing_ops),
                                )?;

                                let position = image_table_offset_to_memory_location(offset);

                                if memory_finalized_set.contains(&position) {
                                    region.assign_advice(
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

                                    region.assign_advice(
                                        || "post image table: init memory lookup",
                                        self.config.memory_finalized_lookup_encode,
                                        offset,
                                        || Ok(bn_to_field::<F>(&address) + *post),
                                    )?;
                                }

                                Ok(entry)
                            })
                            .collect::<Result<Vec<_>, Error>>()
                    }?;

                    Ok(entries)
                };

                let layouter = image_table_assigner.exec(
                    initialization_state_handler,
                    inherited_frame_entries_handler,
                    instruction_handler,
                    br_table_handler,
                    padding_handler,
                    init_memory_entries_handler,
                )?;

                Ok(Some((
                    layouter,
                    Arc::try_unwrap(assigned_rest_memory_finalized_count_cell)
                        .unwrap()
                        .into_inner()
                        .unwrap()
                        .unwrap(),
                )))
            },
        )
    }
}
