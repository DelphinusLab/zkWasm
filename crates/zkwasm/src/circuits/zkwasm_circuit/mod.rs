use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use ark_std::end_timer;
use ark_std::start_timer;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::floor_planner::FlatFloorPlanner;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use log::debug;
use log::info;
use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::CalledFrameTable;
use specs::jtable::INHERITED_FRAME_TABLE_ENTRIES;
use specs::slice::FrameTableSlice;
use specs::slice::Slice;

use crate::circuits::bit_table::BitTableChip;
use crate::circuits::bit_table::BitTableConfig;
use crate::circuits::bit_table::BitTableTrait;
use crate::circuits::compute_slice_capability;
use crate::circuits::etable::EventTableChip;
use crate::circuits::etable::EventTableConfig;
use crate::circuits::external_host_call_table::ExternalHostCallChip;
use crate::circuits::external_host_call_table::ExternalHostCallTableConfig;
use crate::circuits::image_table::compute_maximal_pages;
use crate::circuits::image_table::ImageTableChip;
use crate::circuits::jtable::JumpTableChip;
use crate::circuits::jtable::JumpTableConfig;
use crate::circuits::mtable::MemoryTableChip;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::post_image_table::PostImageTableChip;
use crate::circuits::rtable::RangeTableChip;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::utils::image_table::EncodeImageTable;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::exec_with_profile;
use crate::foreign::context::circuits::assign::ContextContHelperTableChip;
use crate::foreign::context::circuits::ContextContHelperTableConfig;
use crate::foreign::context::circuits::CONTEXT_FOREIGN_TABLE_KEY;
use crate::foreign::foreign_table_enable_lines;
use crate::foreign::wasm_input_helper::circuits::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::WASM_INPUT_FOREIGN_TABLE_KEY;
use crate::foreign::ForeignTableConfig;
use crate::runtime::memory_event_of_step;

use super::etable::assign::EventTablePermutationCells;
use super::image_table::ImageTableConfig;
use super::jtable::FrameEtablePermutationCells;
use super::post_image_table::PostImageTableConfig;
use super::LastSliceCircuit;
use super::OngoingCircuit;

pub const VAR_COLUMNS: usize = 40;

// Reserve 128 rows(greater than step size of all tables) to keep usable rows away from
//   blind rows and range checking rows.
// Reserve (1 << 16) / 2 to allow u16 range checking based on shuffle with step 2.
pub(crate) const RESERVE_ROWS: usize = 128 + (1 << 15);

#[derive(Default, Clone)]
struct AssignedCells<F: FieldExt> {
    pre_image_table_cells: Arc<Mutex<Option<ImageTableLayouter<AssignedCell<F, F>>>>>,
    post_image_table_cells:
        Arc<Mutex<Option<Option<(ImageTableLayouter<AssignedCell<F, F>>, AssignedCell<F, F>)>>>>,
    mtable_rest_mops: Arc<Mutex<Option<AssignedCell<F, F>>>>,
    rest_memory_finalize_ops_cell: Arc<Mutex<Option<Option<AssignedCell<F, F>>>>>,
    etable_cells: Arc<Mutex<Option<EventTablePermutationCells<F>>>>,
    rest_ops_cell_in_frame_table: Arc<Mutex<Option<FrameEtablePermutationCells<F>>>>,
    inherited_frame_entry_in_frame_table:
        Arc<Mutex<Option<Box<[AssignedCell<F, F>; INHERITED_FRAME_TABLE_ENTRIES]>>>>,
}

#[derive(Clone)]
pub struct ZkWasmCircuitConfig<F: FieldExt> {
    shuffle_range_check_helper: (Column<Fixed>, Column<Fixed>, Column<Fixed>),
    rtable: RangeTableConfig<F>,
    image_table: ImageTableConfig<F>,
    post_image_table: PostImageTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    frame_table: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    bit_table: BitTableConfig<F>,
    external_host_call_table: ExternalHostCallTableConfig<F>,
    context_helper_table: ContextContHelperTableConfig<F>,

    foreign_table_from_zero_index: Column<Fixed>,

    blinding_factors: usize,
}

macro_rules! impl_zkwasm_circuit {
    ($name:ident, $last_slice:expr) => {
        impl<F: FieldExt> Circuit<F> for $name<F> {
            type Config = ZkWasmCircuitConfig<F>;

            type FloorPlanner = FlatFloorPlanner;

            fn without_witnesses(&self) -> Self {
                $name::new(
                    self.k,
                    // fill slice like circuit_without_witness
                    Slice {
                        itable: self.slice.itable.clone(),
                        br_table: self.slice.br_table.clone(),
                        elem_table: self.slice.elem_table.clone(),
                        configure_table: self.slice.configure_table.clone(),
                        initial_frame_table: self.slice.initial_frame_table.clone(),

                        etable: Arc::new(EventTable::default()),
                        frame_table: Arc::new(FrameTableSlice {
                            inherited: self.slice.initial_frame_table.clone(),
                            called: CalledFrameTable::default(),
                        }),
                        post_inherited_frame_table: self.slice.initial_frame_table.clone(),

                        imtable: self.slice.imtable.clone(),
                        post_imtable: self.slice.imtable.clone(),

                        initialization_state: self.slice.initialization_state.clone(),
                        post_initialization_state: self.slice.initialization_state.clone(),

                        external_host_call_table: ExternalHostCallTable::default().into(),
                        context_input_table: Arc::new(Vec::new()),
                        context_output_table: Arc::new(Vec::new()),

                        is_last_slice: self.slice.is_last_slice,
                    },
                )
                .unwrap()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                /*
                 * Allocate a column to enable assign_advice_from_constant.
                 */
                {
                    let constants = meta.fixed_column();
                    meta.enable_constant(constants);
                    meta.enable_equality(constants);
                }

                let (l_0, l_active, l_active_last) = (
                    meta.fixed_column(),
                    meta.fixed_column(),
                    meta.fixed_column(),
                );

                let memory_addr_sel = if cfg!(feature = "continuation") {
                    Some(meta.fixed_column())
                } else {
                    None
                };

                let foreign_table_from_zero_index = meta.fixed_column();

                let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

                let rtable = RangeTableConfig::configure(meta);
                let image_table = ImageTableConfig::configure(meta, memory_addr_sel);
                let mtable = MemoryTableConfig::configure(
                    meta,
                    (l_0, l_active, l_active_last),
                    &mut cols,
                    &rtable,
                    &image_table,
                );
                let frame_table = JumpTableConfig::configure(meta, $last_slice);
                let post_image_table = PostImageTableConfig::configure(
                    meta,
                    memory_addr_sel,
                    &mtable,
                    &frame_table,
                    &image_table,
                );
                let external_host_call_table = ExternalHostCallTableConfig::configure(meta);
                let bit_table = BitTableConfig::configure(meta, &rtable);

                let wasm_input_helper_table =
                    WasmInputHelperTableConfig::configure(meta, foreign_table_from_zero_index);
                let context_helper_table =
                    ContextContHelperTableConfig::configure(meta, foreign_table_from_zero_index);

                let mut foreign_table_configs: BTreeMap<_, Box<(dyn ForeignTableConfig<F>)>> =
                    BTreeMap::new();
                foreign_table_configs.insert(
                    WASM_INPUT_FOREIGN_TABLE_KEY,
                    Box::new(wasm_input_helper_table.clone()),
                );
                foreign_table_configs.insert(
                    CONTEXT_FOREIGN_TABLE_KEY,
                    Box::new(context_helper_table.clone()),
                );

                let etable = EventTableConfig::configure(
                    meta,
                    (l_0, l_active, l_active_last),
                    &mut cols,
                    &rtable,
                    &image_table,
                    &mtable,
                    &frame_table,
                    &bit_table,
                    &external_host_call_table,
                    &foreign_table_configs,
                );

                assert_eq!(cols.count(), 0);

                Self::Config {
                    shuffle_range_check_helper: (l_0, l_active, l_active_last),
                    rtable,
                    image_table,
                    post_image_table,
                    mtable,
                    frame_table,
                    etable,
                    bit_table,
                    external_host_call_table,
                    context_helper_table,
                    foreign_table_from_zero_index,

                    blinding_factors: meta.blinding_factors(),
                }
            }

            fn synthesize(
                &self,
                config: Self::Config,
                layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                let timer = start_timer!(|| "Prepare assignment");

                let l_last = (1 << self.k) - (config.blinding_factors + 1);
                let max_available_rows =
                    (1 << self.k) - (config.blinding_factors + 1 + RESERVE_ROWS);
                debug!("max_available_rows: {:?}", max_available_rows);

                let circuit_maximal_pages = compute_maximal_pages(self.k);
                info!(
                    "Circuit K: {} supports up to {} pages.",
                    self.k, circuit_maximal_pages
                );

                let rchip = RangeTableChip::new(config.rtable);
                let image_chip = ImageTableChip::new(config.image_table);
                let post_image_chip = PostImageTableChip::new(config.post_image_table);
                let mchip = MemoryTableChip::new(config.mtable, max_available_rows);
                let frame_table_chip = JumpTableChip::new(config.frame_table, max_available_rows);
                let echip = EventTableChip::new(
                    config.etable,
                    compute_slice_capability(self.k) as usize,
                    max_available_rows,
                );
                let bit_chip = BitTableChip::new(config.bit_table, max_available_rows);
                let external_host_call_chip =
                    ExternalHostCallChip::new(config.external_host_call_table, max_available_rows);
                let context_chip = ContextContHelperTableChip::new(config.context_helper_table);

                let image_table_assigner = exec_with_profile!(|| "Prepare image table assigner", {
                    ImageTableAssigner::new(
                        // Add one for default lookup value
                        self.slice.itable.len() + 1,
                        self.slice.br_table.entries().len()
                            + self.slice.elem_table.entries().len()
                            + 1,
                        circuit_maximal_pages,
                    )
                });

                let memory_writing_table: MemoryWritingTable = exec_with_profile!(
                    || "Prepare mtable",
                    MemoryWritingTable::from(
                        self.k,
                        self.slice.create_memory_table(memory_event_of_step),
                    )
                );

                let etable = exec_with_profile!(
                    || "Prepare memory info for etable",
                    EventTableWithMemoryInfo::new(&self.slice.etable, &memory_writing_table,)
                );

                let assigned_cells = AssignedCells::default();

                let layouter_cloned = layouter.clone();
                let assigned_cells_cloned = assigned_cells.clone();
                end_timer!(timer);

                let timer = start_timer!(|| "Assign");
                rayon::scope(move |s| {
                    let memory_writing_table = Arc::new(memory_writing_table);
                    let etable = Arc::new(etable);

                    let _layouter = layouter.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Init range chip", {
                            let (l_0, l_active, l_active_last) = config.shuffle_range_check_helper;

                            _layouter
                                .assign_region(
                                    || "range check sel helper",
                                    |region| {
                                        region
                                            .assign_fixed(|| "l_0", l_0, 0, || Ok(F::one()))
                                            .unwrap();

                                        region
                                            .assign_fixed(
                                                || "l_active_last",
                                                l_active_last,
                                                l_last - 1,
                                                || Ok(F::one()),
                                            )
                                            .unwrap();

                                        for offset in 0..l_last {
                                            region
                                                .assign_fixed(
                                                    || "l_active_last",
                                                    l_active,
                                                    offset,
                                                    || Ok(F::one()),
                                                )
                                                .unwrap();
                                        }

                                        Ok(())
                                    },
                                )
                                .unwrap();

                            rchip.init(_layouter, self.k).unwrap()
                        });
                    });

                    let _layouter = layouter.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(
                            || "Init foreign table index",
                            _layouter
                                .assign_region(
                                    || "foreign helper",
                                    |region| {
                                        for offset in 0..foreign_table_enable_lines(self.k) {
                                            region.assign_fixed(
                                                || "foreign table from zero index",
                                                config.foreign_table_from_zero_index,
                                                offset,
                                                || Ok(F::from(offset as u64)),
                                            )?;
                                        }

                                        Ok(())
                                    },
                                )
                                .unwrap()
                        );
                    });

                    let _layouter = layouter.clone();
                    let _etable = etable.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(
                            || "Assign bit table",
                            bit_chip
                                .assign(_layouter, _etable.filter_bit_table_entries())
                                .unwrap()
                        );
                    });

                    let _layouter = layouter.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(
                            || "Assign external host call table",
                            external_host_call_chip
                                .assign(_layouter, &self.slice.external_host_call_table,)
                                .unwrap()
                        );
                    });

                    let _layouter = layouter.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(
                            || "Assign context cont chip",
                            context_chip
                                .assign(
                                    _layouter,
                                    &self.slice.context_input_table,
                                    &self.slice.context_output_table
                                )
                                .unwrap()
                        );
                    });

                    let _layouter = layouter.clone();
                    let _assigned_cells = assigned_cells.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Assign pre image table chip", {
                            let pre_image_table =
                                self.slice.encode_pre_compilation_table_values(self.k);

                            let cells = image_chip
                                .assign(_layouter, &image_table_assigner, pre_image_table)
                                .unwrap();

                            *_assigned_cells.pre_image_table_cells.lock().unwrap() = Some(cells);
                        });
                    });

                    let _layouter = layouter.clone();
                    let _assigned_cells = assigned_cells.clone();
                    let _memory_writing_table = memory_writing_table.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Assign post image table chip", {
                            let post_image_table: ImageTableLayouter<F> =
                                self.slice.encode_post_compilation_table_values(self.k);

                            let (rest_memory_writing_ops, memory_finalized_set) =
                                _memory_writing_table.count_rest_memory_finalize_ops();

                            let cells = post_image_chip
                                .assign(
                                    _layouter,
                                    &image_table_assigner,
                                    post_image_table,
                                    rest_memory_writing_ops,
                                    memory_finalized_set,
                                )
                                .unwrap();

                            *_assigned_cells.post_image_table_cells.lock().unwrap() = Some(cells);
                        });
                    });

                    let _layouter = layouter.clone();
                    let _assigned_cells = assigned_cells.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Assign frame table", {
                            let (rest_ops_cell, inherited_frame_entry_cells) = frame_table_chip
                                .assign(_layouter, &self.slice.frame_table)
                                .unwrap();

                            *_assigned_cells.rest_ops_cell_in_frame_table.lock().unwrap() =
                                Some(rest_ops_cell);
                            *_assigned_cells
                                .inherited_frame_entry_in_frame_table
                                .lock()
                                .unwrap() = Some(inherited_frame_entry_cells);
                        });
                    });

                    let _layouter = layouter.clone();
                    let _assigned_cells = assigned_cells.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Assign mtable", {
                            let (rest_mops, rest_memory_finalize_ops_cell) =
                                mchip.assign(_layouter, &memory_writing_table).unwrap();

                            *_assigned_cells.mtable_rest_mops.lock().unwrap() = Some(rest_mops);
                            *_assigned_cells
                                .rest_memory_finalize_ops_cell
                                .lock()
                                .unwrap() = Some(rest_memory_finalize_ops_cell);
                        });
                    });

                    let _layouter = layouter.clone();
                    let _assigned_cells = assigned_cells.clone();
                    s.spawn(move |_| {
                        exec_with_profile!(|| "Assign etable", {
                            let cells = echip
                                .assign(
                                    _layouter,
                                    &self.slice.itable,
                                    &etable,
                                    &self.slice.configure_table,
                                    &self.slice.frame_table,
                                    &self.slice.initialization_state,
                                    &self.slice.post_initialization_state,
                                    self.slice.is_last_slice,
                                )
                                .unwrap();

                            *_assigned_cells.etable_cells.lock().unwrap() = Some(cells);
                        });
                    });
                });
                end_timer!(timer);

                macro_rules! into_inner {
                    ($arc:ident) => {
                        let $arc = Arc::try_unwrap(assigned_cells_cloned.$arc)
                            .unwrap()
                            .into_inner()
                            .unwrap()
                            .unwrap();
                    };
                }

                into_inner!(inherited_frame_entry_in_frame_table);
                into_inner!(etable_cells);
                into_inner!(mtable_rest_mops);
                into_inner!(rest_memory_finalize_ops_cell);
                into_inner!(pre_image_table_cells);
                into_inner!(post_image_table_cells);
                into_inner!(rest_ops_cell_in_frame_table);
                /*
                 * Permutation between chips
                 */
                let timer = start_timer!(|| "permutation");
                layouter_cloned.assign_region(
                    || "permutation between tables",
                    |region| {
                        // 1. inherited frame entries
                        // 1.1. between frame table and pre image table
                        for (left, right) in inherited_frame_entry_in_frame_table
                            .iter()
                            .zip(pre_image_table_cells.inherited_frame_entries.iter())
                        {
                            region.constrain_equal(left.cell(), right.cell())?;
                        }

                        // 2. rest jops between event chip and frame chip
                        region.constrain_equal(
                            etable_cells.rest_jops.rest_call_ops.cell(),
                            rest_ops_cell_in_frame_table.rest_call_ops.cell(),
                        )?;

                        region.constrain_equal(
                            etable_cells.rest_jops.rest_return_ops.cell(),
                            rest_ops_cell_in_frame_table.rest_return_ops.cell(),
                        )?;

                        // 3. rest_mops between event chip and memory chip
                        region.constrain_equal(
                            etable_cells.rest_mops.cell(),
                            mtable_rest_mops.cell(),
                        )?;

                        // 4. (if continuation) memory finalized count between memory chip and post image chip
                        if let Some((_, rest_memory_finalized_ops_in_post_image_table)) =
                            post_image_table_cells.as_ref()
                        {
                            region.constrain_equal(
                                rest_memory_finalized_ops_in_post_image_table.cell(),
                                rest_memory_finalize_ops_cell.as_ref().unwrap().cell(),
                            )?;
                        }

                        // 5. initialization state
                        // 5.1 between event chip and pre image chip
                        etable_cells
                            .pre_initialization_state
                            .zip_for_each(&pre_image_table_cells.initialization_state, |l, r| {
                                region.constrain_equal(l.cell(), r.cell())
                            })?;

                        // 5.2 (if continuation) between event chip and post image chip
                        if let Some((post_image_table_cells, _)) = post_image_table_cells.as_ref() {
                            etable_cells.post_initialization_state.zip_for_each(
                                &post_image_table_cells.initialization_state,
                                |l, r| region.constrain_equal(l.cell(), r.cell()),
                            )?;
                        }

                        // 6. fixed part(instructions, br_tables, padding) within pre image chip and post image chip
                        if let Some((post_image_table_cells, _)) = post_image_table_cells.as_ref() {
                            for (l, r) in pre_image_table_cells
                                .instructions
                                .iter()
                                .zip(post_image_table_cells.instructions.iter())
                            {
                                region.constrain_equal(l.cell(), r.cell())?;
                            }

                            for (l, r) in pre_image_table_cells
                                .br_table_entires
                                .iter()
                                .zip(post_image_table_cells.br_table_entires.iter())
                            {
                                region.constrain_equal(l.cell(), r.cell())?;
                            }

                            for (l, r) in pre_image_table_cells
                                .padding_entires
                                .iter()
                                .zip(post_image_table_cells.padding_entires.iter())
                            {
                                region.constrain_equal(l.cell(), r.cell())?;
                            }
                        }

                        Ok(())
                    },
                )?;
                end_timer!(timer);

                Ok(())
            }
        }
    };
}

impl_zkwasm_circuit!(OngoingCircuit, false);
impl_zkwasm_circuit!(LastSliceCircuit, true);
