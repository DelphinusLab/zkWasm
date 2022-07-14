use crate::circuits::{
    etable::{EventTableChip, EventTableConfig},
    imtable::InitMemoryTableConfig,
    itable::{InstructionTableChip, InstructionTableConfig},
    jtable::{JumpTableChip, JumpTableConfig},
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
        let jtable = JumpTableConfig::configure(meta, &mut cols, &rtable);
        let mtable = MemoryTableConfig::configure(meta, &mut cols, &rtable, &imtable);
        let etable =
            EventTableConfig::configure(meta, &mut cols, &rtable, &itable, &mtable, &jtable);

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
        let echip = EventTableChip::new(config.etable);
        let rchip = RangeTableChip::new(config.rtable);
        let ichip = InstructionTableChip::new(config.itable);
        let mchip = MemoryTableChip::new(config.mtable);
        let jchip = JumpTableChip::new(config.jtable);

        println!("etable length is {}", self.execution_tables.etable.len());
        println!(
            "mtable length is {}",
            self.execution_tables.mtable.entries().len()
        );

        rchip.init(&mut layouter, 16usize)?;
        ichip.assign(&mut layouter, &self.compile_tables.itable)?;

        layouter.assign_region(
            || "table",
            |region| {
                let mut ctx = Context::new(region);
                let (rest_mops_cell, rest_jops_cell) = echip.assign(&mut ctx, &self.execution_tables.etable)?;

                ctx.reset();
                mchip.assign(&mut ctx, &self.execution_tables.mtable.entries(), rest_mops_cell)?;
                ctx.reset();
                jchip.assign(&mut ctx, &self.execution_tables.jtable, rest_jops_cell)?;
                Ok(())
            },
        )?;

        Ok(())
    }
}
