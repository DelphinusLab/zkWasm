use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use ark_std::end_timer;
use ark_std::start_timer;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::floor_planner::FlatFloorPlanner;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use log::debug;
use log::info;
use specs::ExecutionTable;
use specs::Tables;

use crate::circuits::bit_table::BitTableChip;
use crate::circuits::bit_table::BitTableConfig;
use crate::circuits::bit_table::BitTableTrait;
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
use crate::circuits::utils::image_table::EncodeCompilationTableValues;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::ZkWasmCircuit;
use crate::exec_with_profile;
use crate::foreign::context::circuits::assign::ContextContHelperTableChip;
use crate::foreign::context::circuits::assign::ExtractContextFromTrace;
use crate::foreign::context::circuits::ContextContHelperTableConfig;
use crate::foreign::context::circuits::CONTEXT_FOREIGN_TABLE_KEY;
use crate::foreign::foreign_table_enable_lines;
use crate::foreign::wasm_input_helper::circuits::assign::ExtractInputFromTrace;
use crate::foreign::wasm_input_helper::circuits::assign::WasmInputHelperTableChip;
use crate::foreign::wasm_input_helper::circuits::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::WASM_INPUT_FOREIGN_TABLE_KEY;
use crate::foreign::ForeignTableConfig;
use crate::runtime::memory_event_of_step;

use super::config::zkwasm_k;
use super::image_table::ImageTableConfig;
use super::post_image_table::PostImageTableConfig;

pub const VAR_COLUMNS: usize = if cfg!(feature = "continuation") {
    60
} else {
    51
};

// Reserve a few rows to keep usable rows away from blind rows.
// The maximal step size of all tables is bit_table::STEP_SIZE.
pub(crate) const RESERVE_ROWS: usize = crate::circuits::bit_table::STEP_SIZE;

#[derive(Clone)]
pub struct ZkWasmCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    image_table: ImageTableConfig<F>,
    post_image_table: PostImageTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    bit_table: BitTableConfig<F>,
    external_host_call_table: ExternalHostCallTableConfig<F>,
    context_helper_table: ContextContHelperTableConfig<F>,
    wasm_input_helper_table: WasmInputHelperTableConfig<F>,

    foreign_table_from_zero_index: Column<Fixed>,

    max_available_rows: usize,
    circuit_maximal_pages: u32,

    k: u32,
}

impl<F: FieldExt> Circuit<F> for ZkWasmCircuit<F> {
    type Config = ZkWasmCircuitConfig<F>;

    type FloorPlanner = FlatFloorPlanner;

    fn without_witnesses(&self) -> Self {
        ZkWasmCircuit::new(
            Tables {
                compilation_tables: self.tables.compilation_tables.clone(),
                execution_tables: ExecutionTable::default(),
                post_image_table: self.tables.post_image_table.clone(),
                is_last_slice: self.tables.is_last_slice,
            },
            self.slice_capability,
        )
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let k = zkwasm_k();

        /*
         * Allocate a column to enable assign_advice_from_constant.
         */
        {
            let constants = meta.fixed_column();
            meta.enable_constant(constants);
            meta.enable_equality(constants);
        }

        let memory_addr_sel = if cfg!(feature = "continuation") {
            Some(meta.fixed_column())
        } else {
            None
        };

        let foreign_table_from_zero_index = meta.fixed_column();

        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

        let rtable = RangeTableConfig::configure(meta);
        let image_table = ImageTableConfig::configure(meta, memory_addr_sel);
        let mtable = MemoryTableConfig::configure(meta, k, &mut cols, &rtable, &image_table);
        let post_image_table =
            PostImageTableConfig::configure(meta, memory_addr_sel, &mtable, &image_table);
        let jtable = JumpTableConfig::configure(meta, &mut cols);
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
            k,
            &mut cols,
            &rtable,
            &image_table,
            &mtable,
            &jtable,
            &bit_table,
            &external_host_call_table,
            &foreign_table_configs,
        );

        assert_eq!(cols.count(), 0);

        let max_available_rows = (1 << k) - (meta.blinding_factors() + 1 + RESERVE_ROWS);
        debug!("max_available_rows: {:?}", max_available_rows);

        let circuit_maximal_pages = compute_maximal_pages(k);
        info!(
            "Circuit K: {} supports up to {} pages.",
            k, circuit_maximal_pages
        );

        Self::Config {
            rtable,
            image_table,
            post_image_table,
            mtable,
            jtable,
            etable,
            bit_table,
            external_host_call_table,
            context_helper_table,
            wasm_input_helper_table,
            foreign_table_from_zero_index,

            max_available_rows,
            circuit_maximal_pages,

            k,
        }
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let assign_timer = start_timer!(|| "Assign");

        let rchip = RangeTableChip::new(config.rtable);
        let image_chip = ImageTableChip::new(config.image_table);
        let post_image_chip = PostImageTableChip::new(config.post_image_table);
        let mchip = MemoryTableChip::new(config.mtable, config.max_available_rows);
        let frame_table_chip = JumpTableChip::new(config.jtable, config.max_available_rows);
        let echip = EventTableChip::new(
            config.etable,
            self.slice_capability,
            config.max_available_rows,
        );
        let bit_chip = BitTableChip::new(config.bit_table, config.max_available_rows);
        let external_host_call_chip =
            ExternalHostCallChip::new(config.external_host_call_table, config.max_available_rows);
        let wasm_input_helper_chip = WasmInputHelperTableChip::new(config.wasm_input_helper_table);
        let context_chip = ContextContHelperTableChip::new(config.context_helper_table);

        let image_table_assigner = ImageTableAssigner::new(
            // Add one for default lookup value
            self.tables.compilation_tables.itable.len() + 1,
            self.tables.compilation_tables.br_table.entries().len()
                + self.tables.compilation_tables.elem_table.entries().len()
                + 1,
            config.circuit_maximal_pages,
        );

        let memory_writing_table: Arc<MemoryWritingTable> = Arc::new(MemoryWritingTable::from(
            config.k,
            self.tables.create_memory_table(memory_event_of_step),
        ));
        let memory_writing_table_for_post_image_table = memory_writing_table.clone();

        let etable = Arc::new(exec_with_profile!(
            || "Prepare memory info for etable",
            EventTableWithMemoryInfo::new(
                &self.tables.execution_tables.etable,
                &memory_writing_table,
            )
        ));
        let etable_for_bit_table = etable.clone();

        let layout1 = layouter.clone();
        let layout2 = layouter.clone();
        let layout22 = layouter.clone();
        let layout3 = layouter.clone();
        let layout4 = layouter.clone();
        let layout5 = layouter.clone();
        let layout55 = layouter.clone();
        let layout6 = layouter.clone();
        let layout7 = layouter.clone();
        let layout8 = layouter.clone();

        let assigned_pre_image_table_cells = Arc::new(Mutex::new(None));
        let assigned_post_image_table_cells = Arc::new(Mutex::new(None));
        let assigned_mtable_rest_mops = Arc::new(Mutex::new(None));
        let assigned_rest_memory_finalize_ops_cell = Arc::new(Mutex::new(None));
        let assigned_etable_cells = Arc::new(Mutex::new(None));
        let assigned_rest_jops_cell_in_frame_table = Arc::new(Mutex::new(None));
        let assigned_static_frame_entry_in_frame_table = Arc::new(Mutex::new(None));

        let assigned_pre_image_table_cells_assignment_pass = assigned_pre_image_table_cells.clone();
        let assigned_post_image_table_cells_assignment_pass =
            assigned_post_image_table_cells.clone();
        let assigned_mtable_rest_mops_assignment_pass = assigned_mtable_rest_mops.clone();
        let assigned_rest_memory_finalize_ops_cell_assignment_pass =
            assigned_rest_memory_finalize_ops_cell.clone();
        let assigned_etable_cells_assignment_pass = assigned_etable_cells.clone();
        let assigned_rest_jops_cell_in_frame_table_assignment_pass =
            assigned_rest_jops_cell_in_frame_table.clone();
        let assigned_static_frame_entry_in_frame_table_assignment_pass =
            assigned_static_frame_entry_in_frame_table.clone();

        rayon::scope(|s| {
            s.spawn(move |_| {
                exec_with_profile!(
                    || "Init range chip",
                    rchip.init(&layout1, config.k).unwrap()
                );
            });

            s.spawn(move |_| {
                exec_with_profile!(
                    || "Init foreign table index",
                    layout2
                        .assign_region(
                            || "foreign helper",
                            |region| {
                                for offset in 0..foreign_table_enable_lines(config.k) {
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

            s.spawn(move |_| {
                exec_with_profile!(
                    || "Assign bit table",
                    bit_chip
                        .assign(&layout22, etable_for_bit_table.filter_bit_table_entries())
                        .unwrap()
                );
            });

            s.spawn(move |_| {
                exec_with_profile!(
                    || "Assign external host call table",
                    external_host_call_chip
                        .assign(
                            &layout3,
                            &self
                                .tables
                                .execution_tables
                                .etable
                                .filter_external_host_call_table(),
                        )
                        .unwrap()
                );
            });

            s.spawn(move |_| {
                exec_with_profile!(|| "Assign context cont chip", {
                    context_chip
                        .assign(
                            &layout4,
                            &self.tables.execution_tables.etable.get_context_inputs(),
                            &self.tables.execution_tables.etable.get_context_outputs(),
                        )
                        .unwrap();

                    wasm_input_helper_chip
                        .assign(
                            &layout4,
                            self.tables.execution_tables.etable.get_wasm_inputs(),
                        )
                        .unwrap();
                });
            });

            s.spawn(move |_| {
                let pre_image_table = self
                    .tables
                    .compilation_tables
                    .encode_compilation_table_values(config.circuit_maximal_pages);

                let cells = exec_with_profile!(
                    || "Assign pre image table chip",
                    image_chip
                        .assign(&layout5, &image_table_assigner, pre_image_table)
                        .unwrap()
                );

                *assigned_pre_image_table_cells_assignment_pass
                    .lock()
                    .unwrap() = Some(cells);
            });

            s.spawn(move |_| {
                let post_image_table: ImageTableLayouter<F> = self
                    .tables
                    .post_image_table
                    .encode_compilation_table_values(config.circuit_maximal_pages);

                let (rest_memory_writing_ops, memory_finalized_set) =
                    memory_writing_table_for_post_image_table.count_rest_memory_finalize_ops();

                let cells = post_image_chip
                    .assign(
                        &layout55,
                        &image_table_assigner,
                        post_image_table,
                        rest_memory_writing_ops,
                        memory_finalized_set,
                    )
                    .unwrap();

                *assigned_post_image_table_cells_assignment_pass
                    .lock()
                    .unwrap() = Some(cells);
            });

            s.spawn(move |_| {
                exec_with_profile!(|| "Assign frame table", {
                    let (rest_jops_cell, static_frame_entry_cells) = frame_table_chip
                        .assign(
                            &layout8,
                            &self.tables.compilation_tables.static_jtable,
                            &self.tables.execution_tables.jtable,
                        )
                        .unwrap();

                    *assigned_rest_jops_cell_in_frame_table_assignment_pass
                        .lock()
                        .unwrap() = Some(rest_jops_cell);
                    *assigned_static_frame_entry_in_frame_table_assignment_pass
                        .lock()
                        .unwrap() = Some(static_frame_entry_cells);
                });
            });

            s.spawn(move |_| {
                exec_with_profile!(|| "Assign mtable", {
                    let (rest_mops, rest_memory_finalize_ops_cell) =
                        mchip.assign(&layout7, memory_writing_table).unwrap();

                    *assigned_mtable_rest_mops_assignment_pass.lock().unwrap() = Some(rest_mops);
                    *assigned_rest_memory_finalize_ops_cell_assignment_pass
                        .lock()
                        .unwrap() = Some(rest_memory_finalize_ops_cell);
                });
            });

            s.spawn(move |_| {
                exec_with_profile!(|| "Assign etable", {
                    let cells = echip
                        .assign(
                            &layout6,
                            &self.tables.compilation_tables.itable,
                            &etable,
                            &self.tables.compilation_tables.configure_table,
                            &self.tables.compilation_tables.initialization_state,
                            &self.tables.post_image_table.initialization_state,
                            self.tables.is_last_slice,
                        )
                        .unwrap();

                    *assigned_etable_cells_assignment_pass.lock().unwrap() = Some(cells);
                });
            });
        });

        fn into_inner<T: std::fmt::Debug>(v: Arc<Mutex<Option<T>>>) -> T {
            Arc::try_unwrap(v).unwrap().into_inner().unwrap().unwrap()
        }

        let assigned_static_frame_entry_in_frame_table =
            into_inner(assigned_static_frame_entry_in_frame_table);
        let assigned_etable_cells = into_inner(assigned_etable_cells);
        let assigned_mtable_rest_mops = into_inner(assigned_mtable_rest_mops);
        let assigned_rest_memory_finalize_ops = into_inner(assigned_rest_memory_finalize_ops_cell);
        let assigned_pre_image_table_cells = into_inner(assigned_pre_image_table_cells);
        let assigned_post_image_table_cells = into_inner(assigned_post_image_table_cells);
        let assigned_rest_jops_cell_in_frame_table =
            into_inner(assigned_rest_jops_cell_in_frame_table);
        /*
         * Permutation between chips
         *
         */
        layouter.assign_region(
            || "permutation between tables",
            |region| {
                // 1. static frame entries
                // 1.1. between frame table and pre image table
                for (left, right) in assigned_static_frame_entry_in_frame_table
                    .iter()
                    .zip(assigned_pre_image_table_cells.static_frame_entries.iter())
                {
                    // enable
                    region.constrain_equal(left.0.cell(), right.0.cell())?;
                    // entry
                    region.constrain_equal(left.1.cell(), right.1.cell())?;
                }

                // 1.2 (if continuation) between frame table and post image table
                if let Some((assigned_post_image_table_cells, _)) =
                    assigned_post_image_table_cells.as_ref()
                {
                    for (left, right) in assigned_static_frame_entry_in_frame_table
                        .iter()
                        .zip(assigned_post_image_table_cells.static_frame_entries.iter())
                    {
                        // enable
                        region.constrain_equal(left.0.cell(), right.0.cell())?;
                        // entry
                        region.constrain_equal(left.1.cell(), right.1.cell())?;
                    }
                }

                // 2. rest jops
                // 2.1 (if not continuation) rest_jops between event chip and frame chip
                if let Some(rest_jops_in_event_chip) = assigned_etable_cells.rest_jops.as_ref() {
                    region.constrain_equal(
                        rest_jops_in_event_chip.cell(),
                        assigned_rest_jops_cell_in_frame_table.cell(),
                    )?;
                }

                // 2.2 (if continuation and last slice circuit) rest_jops between post image chip and frame chip
                #[cfg(feature = "continuation")]
                if self.tables.is_last_slice {
                    if let Some((assigned_post_image_table_cells, _)) =
                        assigned_post_image_table_cells.as_ref()
                    {
                        region.constrain_equal(
                            assigned_post_image_table_cells
                                .initialization_state
                                .jops
                                .cell(),
                            assigned_rest_jops_cell_in_frame_table.cell(),
                        )?;

                        // region.constrain_equal(
                        //     assigned_etable_cells.post_initialization_state.jops.cell(),
                        //     assigned_rest_jops_cell_in_frame_table.cell(),
                        // )?;
                    }
                }

                // 3. rest_mops between event chip and memory chip
                region.constrain_equal(
                    assigned_etable_cells.rest_mops.cell(),
                    assigned_mtable_rest_mops.cell(),
                )?;

                // 4. (if continuation) memory finalized count between memory chip and post image chip
                if let Some((_, rest_memory_finalized_ops_in_post_image_table)) =
                    assigned_post_image_table_cells.as_ref()
                {
                    region.constrain_equal(
                        rest_memory_finalized_ops_in_post_image_table.cell(),
                        assigned_rest_memory_finalize_ops.as_ref().unwrap().cell(),
                    )?;
                }

                // 5. initialization state
                // 5.1 between event chip and pre image chip
                assigned_etable_cells
                    .pre_initialization_state
                    .zip_for_each(
                        &assigned_pre_image_table_cells.initialization_state,
                        |l, r| region.constrain_equal(l.cell(), r.cell()),
                    )?;

                // 5.2 (if continuation) between event chip and post image chip
                if let Some((assigned_post_image_table_cells, _)) =
                    assigned_post_image_table_cells.as_ref()
                {
                    assigned_etable_cells
                        .post_initialization_state
                        .zip_for_each(
                            &assigned_post_image_table_cells.initialization_state,
                            |l, r| region.constrain_equal(l.cell(), r.cell()),
                        )?;
                }

                // 6. fixed part(instructions, br_tables, padding) within pre image chip and post image chip
                if let Some((assigned_post_image_table_cells, _)) =
                    assigned_post_image_table_cells.as_ref()
                {
                    for (l, r) in assigned_pre_image_table_cells
                        .instructions
                        .iter()
                        .zip(assigned_post_image_table_cells.instructions.iter())
                    {
                        region.constrain_equal(l.cell(), r.cell())?;
                    }

                    for (l, r) in assigned_pre_image_table_cells
                        .br_table_entires
                        .iter()
                        .zip(assigned_post_image_table_cells.br_table_entires.iter())
                    {
                        region.constrain_equal(l.cell(), r.cell())?;
                    }

                    for (l, r) in assigned_pre_image_table_cells
                        .padding_entires
                        .iter()
                        .zip(assigned_post_image_table_cells.padding_entires.iter())
                    {
                        region.constrain_equal(l.cell(), r.cell())?;
                    }
                }

                Ok(())
            },
        )?;

        end_timer!(assign_timer);

        Ok(())
    }
}
