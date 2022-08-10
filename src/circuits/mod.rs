use crate::circuits::{
    imtable::{InitMemoryTableConfig, MInitTableChip},
    itable::{InstructionTableChip, InstructionTableConfig},
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
        Expression, ProvingKey, SingleVerifier, VerifyingKey, VirtualCells,
    },
    poly::commitment::{Params, ParamsVerifier},
    transcript::{Blake2bRead, Blake2bWrite, Challenge255},
};
use num_bigint::BigUint;
use rand::rngs::OsRng;
use specs::{CompileTable, ExecutionTable};
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    marker::PhantomData,
    path::PathBuf,
};

use self::{
    jtable::{JumpTableChip, JumpTableConfig},
    mtable_compact::{MemoryTableChip, MemoryTableConfig},
};

pub mod etable_compact;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable_compact;
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

        Self::Config {
            rtable,
            itable,
            imtable,
            mtable,
            jtable,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let rchip = RangeTableChip::new(config.rtable);
        let ichip = InstructionTableChip::new(config.itable);
        let imchip = MInitTableChip::new(config.imtable);
        let mchip = MemoryTableChip::new(config.mtable);
        let jchip = JumpTableChip::new(config.jtable);

        rchip.init(&mut layouter)?;
        ichip.assign(&mut layouter, &self.compile_tables.itable)?;
        if self.compile_tables.imtable.0.len() > 0 {
            imchip.assign(&mut layouter, &self.compile_tables.imtable.0)?;
        }

        layouter.assign_region(
            || "mtable",
            |region| {
                let mut ctx = Context::new(region);
                mchip.assign(&mut ctx, &self.execution_tables.mtable, None)?;
                Ok(())
            },
        )?;

        layouter.assign_region(
            || "jtable",
            |region| {
                let mut ctx = Context::new(region);
                jchip.assign(&mut ctx, &self.execution_tables.jtable, None)?;
                Ok(())
            },
        )?;

        Ok(())
    }
}

trait Encode {
    fn encode(&self) -> BigUint;
}

pub(self) trait Lookup<F: FieldExt> {
    fn encode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;

    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), self.encode(meta))]);
    }
}

pub struct ZkWasmCircuitBuilder {
    pub compile_tables: CompileTable,
    pub execution_tables: ExecutionTable,
}

const PARAMS: &str = "param.data";
const VK: &str = "vk.data";
const PROOF: &str = "proof.data";

impl ZkWasmCircuitBuilder {
    fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
        TestCircuit::new(self.compile_tables.clone(), self.execution_tables.clone())
    }

    fn prepare_param(&self) -> Params<G1Affine> {
        let path = PathBuf::from(PARAMS);

        if path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();
            Params::<G1Affine>::read(Cursor::new(buf)).unwrap()
        } else {
            // Initialize the polynomial commitment parameters
            let timer = start_timer!(|| "build params with K = 18");
            let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(K);
            end_timer!(timer);

            let mut fd = File::create(path.as_path()).unwrap();
            params.write(&mut fd).unwrap();

            params
        }
    }

    fn prepare_vk(
        &self,
        circuit: &TestCircuit<Fr>,
        params: &Params<G1Affine>,
    ) -> VerifyingKey<G1Affine> {
        let path = PathBuf::from(VK);

        if path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();
            VerifyingKey::read::<_, TestCircuit<Fr>>(&mut Cursor::new(buf), params).unwrap()
        } else {
            let timer = start_timer!(|| "build vk");
            let vk = keygen_vk(params, circuit).expect("keygen_vk should not fail");
            end_timer!(timer);

            let mut fd = File::create(path.as_path()).unwrap();
            vk.write(&mut fd).unwrap();

            vk
        }
    }

    fn prepare_pk(
        &self,
        circuit: &TestCircuit<Fr>,
        params: &Params<G1Affine>,
        vk: VerifyingKey<G1Affine>,
    ) -> ProvingKey<G1Affine> {
        let timer = start_timer!(|| "build pk");
        let pk = keygen_pk(&params, vk, circuit).expect("keygen_pk should not fail");
        end_timer!(timer);

        pk
    }

    fn create_proof(
        &self,
        circuits: &[TestCircuit<Fr>],
        params: &Params<G1Affine>,
        pk: &ProvingKey<G1Affine>,
    ) -> Vec<u8> {
        let path = PathBuf::from(PROOF);

        if path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();
            buf
        } else {
            let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);

            let timer = start_timer!(|| "create proof");
            create_proof(params, pk, circuits, &[&[]], OsRng, &mut transcript)
                .expect("proof generation should not fail");
            end_timer!(timer);

            let proof = transcript.finalize();

            let mut fd = File::create(path.as_path()).unwrap();
            fd.write(&proof).unwrap();

            proof
        }
    }

    fn verify_check(
        &self,
        vk: &VerifyingKey<G1Affine>,
        params: &Params<G1Affine>,
        proof: &Vec<u8>,
    ) {
        let public_inputs_size = 0;

        let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();

        let strategy = SingleVerifier::new(&params_verifier);
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

        let timer = start_timer!(|| "verify proof");
        verify_proof(&params_verifier, vk, strategy, &[&[]], &mut transcript).unwrap();
        end_timer!(timer);
    }

    pub fn bench(&self) {
        let circuit: TestCircuit<Fr> = self.build_circuit::<Fr>();

        let params = self.prepare_param();

        let vk = self.prepare_vk(&circuit, &params);
        let pk = self.prepare_pk(&circuit, &params, vk);

        let proof = self.create_proof(&[circuit], &params, &pk);

        self.verify_check(pk.get_vk(), &params, &proof);
    }
}
