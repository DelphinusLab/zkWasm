use ark_std::end_timer;
use ark_std::start_timer;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::SimpleFloorPlanner;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use specs::host_function::HostPlugin;
use specs::ExecutionTable;
use specs::Tables;

use crate::circuits::bit_table::BitTableChip;
use crate::circuits::bit_table::BitTableConfig;
use crate::circuits::brtable::BrTableChip;
use crate::circuits::brtable::BrTableConfig;
use crate::circuits::etable::EventTableChip;
use crate::circuits::etable::EventTableConfig;
use crate::circuits::external_host_call_table::ExternalHostCallChip;
use crate::circuits::external_host_call_table::ExternalHostCallTableConfig;
use crate::circuits::imtable::InitMemoryTableConfig;
use crate::circuits::imtable::InitMemoryTableChip;
use crate::circuits::itable::InstructionTableChip;
use crate::circuits::itable::InstructionTableConfig;
use crate::circuits::jtable::JumpTableChip;
use crate::circuits::jtable::JumpTableConfig;
use crate::circuits::mtable::MemoryTableChip;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::rtable::RangeTableChip;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::utils::Context;
use crate::circuits::TestCircuit;
use crate::circuits::CIRCUIT_CONFIGURE;
use crate::exec_with_profile;
use crate::foreign::wasm_input_helper::circuits::assign::WasmInputHelperTableChip;
use crate::foreign::wasm_input_helper::circuits::WasmInputHelperTableConfig;

pub const VAR_COLUMNS: usize = 43;

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    itable: InstructionTableConfig<F>,
    imtable: InitMemoryTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    brtable: BrTableConfig<F>,
    bit_table: BitTableConfig<F>,
    external_host_call_table: ExternalHostCallTableConfig<F>,
    wasm_input_helper_table: WasmInputHelperTableConfig<F>,
}

impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
    type Config = TestCircuitConfig<F>;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit::new(
            self.fid_of_entry,
            Tables {
                compilation_tables: self.tables.compilation_tables.clone(),
                execution_tables: ExecutionTable::default(),
            },
        )
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let circuit_configure = unsafe { CIRCUIT_CONFIGURE.clone().unwrap() };

        /*
         * Allocate a column to enable assign_advice_from_constant.
         */
        {
            let constants = meta.fixed_column();
            meta.enable_constant(constants);
            meta.enable_equality(constants);
        }

        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

        let rtable = RangeTableConfig::configure([0; 8].map(|_| meta.lookup_table_column()));
        let itable = InstructionTableConfig::configure(meta.lookup_table_column());
        let imtable = InitMemoryTableConfig::configure(meta.lookup_table_column());
        let mtable =
            MemoryTableConfig::configure(meta, &mut cols, &rtable, &imtable, &circuit_configure);
        let jtable = JumpTableConfig::configure(meta, &mut cols);
        let brtable = BrTableConfig::configure(meta.lookup_table_column());
        let external_host_call_table = ExternalHostCallTableConfig::configure(meta);
        let bit_table = BitTableConfig::configure(meta, &rtable);

        let wasm_input_helper_table = WasmInputHelperTableConfig::configure(meta, &rtable);

        let etable = EventTableConfig::configure(
            meta,
            &mut cols,
            &circuit_configure,
            &rtable,
            &itable,
            &mtable,
            &jtable,
            &brtable,
            &bit_table,
            &external_host_call_table,
            &circuit_configure.opcode_selector,
        );

        Self::Config {
            rtable,
            itable,
            imtable,
            mtable,
            jtable,
            etable,
            brtable,
            bit_table,
            external_host_call_table,
            wasm_input_helper_table,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let assign_timer = start_timer!(|| "Assign");

        let rchip = RangeTableChip::new(config.rtable);
        let ichip = InstructionTableChip::new(config.itable);
        let imchip = InitMemoryTableChip::new(config.imtable);
        let mchip = MemoryTableChip::new(config.mtable);
        let jchip = JumpTableChip::new(config.jtable);
        let echip = EventTableChip::new(config.etable);
        let brchip = BrTableChip::new(config.brtable);
        let bit_chip = BitTableChip::new(config.bit_table);
        let external_host_call_chip = ExternalHostCallChip::new(config.external_host_call_table);
        let wasm_input_chip = WasmInputHelperTableChip::new(config.wasm_input_helper_table);

        exec_with_profile!(|| "Init range chip", rchip.init(&mut layouter)?);
        exec_with_profile!(
            || "Init wasm input chip",
            wasm_input_chip.init(&mut layouter)?
        );

        exec_with_profile!(
            || "Assign wasm input chip",
            wasm_input_chip.assign(
                &mut layouter,
                &self
                    .tables
                    .execution_tables
                    .etable
                    .filter_foreign_entries(HostPlugin::HostInput),
            )?
        );

        exec_with_profile!(
            || "Assign instruction table",
            ichip.assign(&mut layouter, &self.tables.compilation_tables.itable)?
        );
        exec_with_profile!(
            || "Assign br table",
            brchip.assign(
                &mut layouter,
                &self.tables.compilation_tables.itable.create_brtable(),
                &self.tables.compilation_tables.elem_table,
            )?
        );

        if self.tables.compilation_tables.imtable.entries().len() > 0 {
            exec_with_profile!(
                || "Assign memory initialization table",
                imchip.assign(&mut layouter, &self.tables.compilation_tables.imtable)?
            );
        }

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

        layouter.assign_region(
            || "jtable mtable etable",
            |region| {
                let mut ctx = Context::new(region);

                let memory_writing_table: MemoryWritingTable =
                    self.tables.execution_tables.mtable.clone().into();

                let etable = exec_with_profile!(
                    || "Prepare memory info for etable",
                    EventTableWithMemoryInfo::new(
                        &self.tables.execution_tables.etable,
                        &memory_writing_table,
                    )
                );

                let (rest_mops_cell, rest_jops_cell) = exec_with_profile!(
                    || "Assign etable",
                    echip.assign(
                        &mut ctx,
                        &etable,
                        &self.tables.compilation_tables.configure_table,
                        self.fid_of_entry,
                    )?
                );

                {
                    ctx.reset();
                    exec_with_profile!(
                        || "Assign mtable",
                        mchip.assign(
                            &mut ctx,
                            rest_mops_cell,
                            &memory_writing_table,
                            self.tables
                                .compilation_tables
                                .imtable
                                .first_consecutive_zero_memory(),
                        )?
                    );
                }

                {
                    ctx.reset();
                    exec_with_profile!(
                        || "Assign frame table",
                        jchip.assign(
                            &mut ctx,
                            &self.tables.execution_tables.jtable,
                            rest_jops_cell,
                            &self.tables.compilation_tables.static_jtable,
                        )?
                    );
                }

                {
                    ctx.reset();
                    exec_with_profile!(|| "Assign bit table", bit_chip.assign(&mut ctx, &etable)?);
                }

                Ok(())
            },
        )?;

        end_timer!(assign_timer);

        Ok(())
    }
}
