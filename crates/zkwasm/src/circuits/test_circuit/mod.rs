use std::collections::BTreeMap;

use ark_std::end_timer;
use ark_std::start_timer;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::SimpleFloorPlanner;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use log::debug;
use specs::ExecutionTable;
use specs::Tables;

use crate::circuits::bit_table::BitTableChip;
use crate::circuits::bit_table::BitTableConfig;
use crate::circuits::etable::EventTableChip;
use crate::circuits::etable::EventTableConfig;
use crate::circuits::external_host_call_table::ExternalHostCallChip;
use crate::circuits::external_host_call_table::ExternalHostCallTableConfig;
use crate::circuits::image_table::EncodeCompilationTableValues;
use crate::circuits::image_table::ImageTableChip;
use crate::circuits::image_table::ImageTableLayouter;
use crate::circuits::jtable::JumpTableChip;
use crate::circuits::jtable::JumpTableConfig;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::rtable::RangeTableChip;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::state_table;
use crate::circuits::state_table::StateTableChip;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::utils::Context;
use crate::circuits::TestCircuit;
use crate::exec_with_profile;
use crate::foreign::context::circuits::assign::ContextContHelperTableChip;
use crate::foreign::context::circuits::assign::ExtractContextFromTrace;
use crate::foreign::context::circuits::ContextContHelperTableConfig;
use crate::foreign::context::circuits::CONTEXT_FOREIGN_TABLE_KEY;
use crate::foreign::foreign_table_enable_lines;
use crate::foreign::wasm_input_helper::circuits::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::WASM_INPUT_FOREIGN_TABLE_KEY;
use crate::foreign::ForeignTableConfig;
use crate::runtime::memory_event_of_step;

use super::config::zkwasm_k;
use super::image_table::ImageTableConfig;
use super::state_table::StateTableConfig;

pub const VAR_COLUMNS: usize = 53;

// Reserve a few rows to keep usable rows away from blind rows.
// The maximal step size of all tables is bit_table::STEP_SIZE.
const RESERVE_ROWS: usize = crate::circuits::bit_table::STEP_SIZE;

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    state_table: StateTableConfig<F>,
    image_table: ImageTableConfig<F>,
    _mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    bit_table: BitTableConfig<F>,
    external_host_call_table: ExternalHostCallTableConfig<F>,
    context_helper_table: ContextContHelperTableConfig<F>,

    foreign_table_from_zero_index: Column<Fixed>,

    max_available_rows: usize,
}

impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
    type Config = TestCircuitConfig<F>;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit::new(Tables {
            compilation_tables: self.tables.compilation_tables.clone(),
            execution_tables: ExecutionTable::default(),
        })
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
        let foreign_table_from_zero_index = meta.fixed_column();

        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

        let rtable = RangeTableConfig::configure(meta);
        let state_table = StateTableConfig::configure(meta);
        let image_table = ImageTableConfig::configure(meta);
        let mtable = MemoryTableConfig::configure(meta, &mut cols, &rtable, &image_table);
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

        #[cfg(feature = "continuation")]
        let state_table = StateTableConfig::configure(meta);

        let max_available_rows = (1 << zkwasm_k()) - (meta.blinding_factors() + 1 + RESERVE_ROWS);
        debug!("max_available_rows: {:?}", max_available_rows);

        Self::Config {
            rtable,
            state_table,
            image_table,
            // TODO: open mtable
            _mtable: mtable,
            jtable,
            etable,
            bit_table,
            external_host_call_table,
            context_helper_table,
            foreign_table_from_zero_index,

            max_available_rows,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let assign_timer = start_timer!(|| "Assign");

        let rchip = RangeTableChip::new(config.rtable);
        let state_chip = StateTableChip::new(config.state_table);
        let image_chip = ImageTableChip::new(config.image_table);
        // TODO: open mtable
        // let mchip = MemoryTableChip::new(config.mtable, config.max_available_rows);
        let jchip = JumpTableChip::new(config.jtable, config.max_available_rows);
        let echip = EventTableChip::new(config.etable, config.max_available_rows);
        let bit_chip = BitTableChip::new(config.bit_table, config.max_available_rows);
        let external_host_call_chip =
            ExternalHostCallChip::new(config.external_host_call_table, config.max_available_rows);
        let context_chip = ContextContHelperTableChip::new(config.context_helper_table);

        layouter.assign_region(
            || "foreign helper",
            |mut region| {
                for offset in 0..foreign_table_enable_lines() {
                    region.assign_fixed(
                        || "foreign table from zero index",
                        config.foreign_table_from_zero_index,
                        offset,
                        || Ok(F::from(offset as u64)),
                    )?;
                }

                Ok(())
            },
        )?;

        exec_with_profile!(|| "Init range chip", rchip.init(&mut layouter)?);

        exec_with_profile!(
            || "Assign external host call table",
            external_host_call_chip.assign(
                &mut layouter,
                &self
                    .tables
                    .execution_tables
                    .etable
                    .filter_external_host_call_table(),
            )?
        );

        let (etable_permutation_cells, static_frame_entries) = layouter.assign_region(
            || "jtable mtable etable",
            |region| {
                let mut ctx = Context::new(region);

                let memory_writing_table: MemoryWritingTable =
                    self.tables.create_memory_table(memory_event_of_step).into();

                let etable = exec_with_profile!(
                    || "Prepare memory info for etable",
                    EventTableWithMemoryInfo::new(
                        &self.tables.execution_tables.etable,
                        &memory_writing_table,
                    )
                );

                let etable_permutation_cells = exec_with_profile!(
                    || "Assign etable",
                    echip.assign(
                        &mut ctx,
                        &etable,
                        &self.tables.compilation_tables.configure_table,
                        &self.tables.execution_tables.initialization_state
                    )?
                );

                // TODO: open mtable
                // {
                //     ctx.reset();
                //     exec_with_profile!(
                //         || "Assign mtable",
                //         mchip.assign(
                //             &mut ctx,
                //             etable_permutation_cells.rest_mops,
                //             &memory_writing_table,
                //             &self.tables.compilation_tables.imtable
                //         )?
                //     );
                // }

                let jtable_info = {
                    ctx.reset();
                    exec_with_profile!(
                        || "Assign frame table",
                        jchip.assign(
                            &mut ctx,
                            &self.tables.execution_tables.jtable,
                            etable_permutation_cells.initialization_state.rest_jops,
                            &self.tables.compilation_tables.static_jtable,
                        )?
                    )
                };

                {
                    ctx.reset();
                    exec_with_profile!(|| "Assign bit table", bit_chip.assign(&mut ctx, &etable)?);
                }

                Ok((etable_permutation_cells, jtable_info))
            },
        )?;

        exec_with_profile!(
            || "Assign context cont chip",
            context_chip.assign(
                &mut layouter,
                &self.tables.execution_tables.etable.get_context_inputs(),
                &self.tables.execution_tables.etable.get_context_outputs()
            )?
        );

        exec_with_profile!(
            || "Assign Image Table",
            image_chip.assign(
                &mut layouter,
                self.tables
                    .compilation_tables
                    .encode_compilation_table_values(),
                ImageTableLayouter {
                    entry_fid: etable_permutation_cells.initialization_state.fid,
                    static_frame_entries,
                    initial_memory_pages: etable_permutation_cells
                        .initialization_state
                        .initial_memory_pages,
                    maximal_memory_pages: etable_permutation_cells.maximal_memory_pages,
                    lookup_entries: None
                }
            )?
        );

        exec_with_profile!(
            || "Assign State Table",
            state_chip.assign(
                &mut layouter,
                &self.tables.execution_tables.initialization_state,
                &etable_permutation_cells.initialization_state
            )?
        );

        end_timer!(assign_timer);

        Ok(())
    }
}
