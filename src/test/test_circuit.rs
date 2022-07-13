use crate::circuits::{
    etable::{EventTableChip, EventTableConfig},
    imtable::{self, InitMemoryTableConfig},
    itable::{InstructionTableChip, InstructionTableConfig},
    jtable::JumpTableConfig,
    mtable::{MemoryTableChip, MemoryTableConfig},
    rtable::{RangeTableChip, RangeTableConfig},
    utils::Context,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Circuit, ConstraintSystem, Error},
};
use specs::{CompileTable, ExecutionTable};
use std::marker::PhantomData;

const VAR_COLUMNS: usize = 50;

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    imtable: InitMemoryTableConfig<F>,
    itable: InstructionTableConfig<F>,
    etable: EventTableConfig<F>,
    jtable: JumpTableConfig<F>,
    mtable: MemoryTableConfig<F>,
}

#[derive(Default)]
pub struct TestCircuit<F: FieldExt> {
    compile_tables: CompileTable,
    execution_tables: ExecutionTable,
    _data: PhantomData<F>,
}

impl<F: FieldExt> TestCircuit<F> {
    pub fn new(compile_tables: CompileTable, execution_tables: ExecutionTable) -> Self {
        TestCircuit {
            compile_tables,
            execution_tables,
            _data: PhantomData,
        }
    }
}

impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
    type Config = TestCircuitConfig<F>;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();
        let rtable = RangeTableConfig::configure([0; 3].map(|_| meta.lookup_table_column()));
        let imtable = InitMemoryTableConfig::configure(meta.lookup_table_column());
        let itable = InstructionTableConfig::configure(meta.lookup_table_column());
        let jtable = JumpTableConfig::configure(&mut cols);
        let mtable = MemoryTableConfig::configure(meta, &mut cols, &rtable, &imtable);
        let etable = EventTableConfig::configure(meta, &mut cols, &itable, &mtable, &jtable);

        Self::Config {
            rtable,
            imtable,
            etable,
            itable,
            jtable,
            mtable,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let _echip = EventTableChip::new(config.etable);
        let rchip = RangeTableChip::new(config.rtable);
        let ichip = InstructionTableChip::new(config.itable);
        let mchip = MemoryTableChip::new(config.mtable);

        rchip.init(&mut layouter, 16usize)?;
        ichip.assign(&mut layouter, &self.compile_tables.itable)?;

        layouter.assign_region(
            || "mtable",
            |region| {
                let mut ctx = Context::new(region);
                mchip.assign(&mut ctx, &self.execution_tables.mtable)?;
                Ok(())
            },
        )?;

        Ok(())
    }
}
