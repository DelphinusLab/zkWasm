use crate::circuits::{
    etable::{EventTableChip, EventTableConfig},
    imtable::{InitMemoryTableConfig, MInitTableChip},
    itable::{InstructionTableChip, InstructionTableConfig},
    jtable::{JumpTableChip, JumpTableConfig},
    mtable::{MemoryTableChip, MemoryTableConfig},
    rtable::{RangeTableChip, RangeTableConfig},
    utils::Context,
};
use ark_std::{end_timer, start_timer};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, SimpleFloorPlanner},
    pairing::bn256::{Bn256, Fr, G1Affine},
    plonk::{
        create_proof, keygen_pk, keygen_vk, verify_proof, Circuit, ConstraintSystem, Error,
        SingleVerifier,
    },
    poly::commitment::{Params, ParamsVerifier},
    transcript::{Blake2bRead, Blake2bWrite, Challenge255},
};
use num_bigint::BigUint;
use rand::rngs::OsRng;
use specs::{CompileTable, ExecutionTable};
use std::marker::PhantomData;

pub mod config_builder;
pub mod etable;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod rtable;
pub mod utils;

const VAR_COLUMNS: usize = 130;
const K: u32 = 18;

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    itable: InstructionTableConfig<F>,
    imtable: InitMemoryTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
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
        let constants = meta.fixed_column();
        meta.enable_constant(constants);
        meta.enable_equality(constants);

        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

        let rtable = RangeTableConfig::configure([0; 10].map(|_| meta.lookup_table_column()));
        let itable = InstructionTableConfig::configure(meta.lookup_table_column());
        let imtable = InitMemoryTableConfig::configure(meta.lookup_table_column());
        let mtable = MemoryTableConfig::configure(meta, &mut cols, &rtable, &imtable);
        let jtable = JumpTableConfig::configure(meta, &mut cols, &rtable);
        let etable =
            EventTableConfig::configure(meta, &mut cols, &rtable, &itable, &mtable, &jtable);

        Self::Config {
            rtable,
            itable,
            imtable,
            mtable,
            jtable,
            etable,
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
        let imchip = MInitTableChip::new(config.imtable);

        rchip.init(&mut layouter)?;
        ichip.assign(&mut layouter, &self.compile_tables.itable)?;
        if self.compile_tables.imtable.0.len() > 0 {
            imchip.assign(&mut layouter, &self.compile_tables.imtable.0)?;
        }

        layouter.assign_region(
            || "table",
            |region| {
                let mut ctx = Context::new(region);
                let (rest_mops_cell, rest_jops_cell) =
                    echip.assign(&mut ctx, &self.execution_tables.etable)?;

                ctx.reset();
                mchip.assign(
                    &mut ctx,
                    &self.execution_tables.mtable.entries(),
                    Some(rest_mops_cell),
                )?;

                ctx.reset();
                jchip.assign(
                    &mut ctx,
                    &self.execution_tables.jtable,
                    Some(rest_jops_cell),
                )?;
                Ok(())
            },
        )?;

        Ok(())
    }
}

trait Encode {
    fn encode(&self) -> BigUint;
}

pub struct ZkWasmCircuitBuilder {
    pub compile_tables: CompileTable,
    pub execution_tables: ExecutionTable,
}

impl ZkWasmCircuitBuilder {
    fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
        TestCircuit::new(self.compile_tables.clone(), self.execution_tables.clone())
    }

    pub fn bench(&self) {
        let public_inputs_size = 0;

        let circuit: TestCircuit<Fr> = self.build_circuit::<Fr>();

        // Initialize the polynomial commitment parameters
        let timer = start_timer!(|| "build params with K = 18");
        let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(K);
        let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
        end_timer!(timer);

        // Initialize the proving key
        let timer = start_timer!(|| "build vk, pk");
        let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &circuit).expect("keygen_pk should not fail");
        end_timer!(timer);

        // Create a proof
        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);

        let timer = start_timer!(|| "create proof");
        create_proof(&params, &pk, &[circuit], &[&[]], OsRng, &mut transcript)
            .expect("proof generation should not fail");
        end_timer!(timer);

        let proof = transcript.finalize();

        let strategy = SingleVerifier::new(&params_verifier);
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

        let timer = start_timer!(|| "verify proof");
        verify_proof(
            &params_verifier,
            pk.get_vk(),
            strategy,
            &[&[]],
            &mut transcript,
        )
        .unwrap();
        end_timer!(timer);
    }
}
