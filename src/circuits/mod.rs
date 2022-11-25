use self::{
    config::{IMTABLE_COLOMNS, VAR_COLUMNS},
    etable_compact::{EventTableChip, EventTableConfig},
    jtable::{JumpTableChip, JumpTableConfig},
    mtable_compact::{MemoryTableChip, MemoryTableConfig},
};
use crate::{
    circuits::{
        config::zkwasm_k,
        imtable::{InitMemoryTableConfig, MInitTableChip},
        itable::{InstructionTableChip, InstructionTableConfig},
        rtable::{RangeTableChip, RangeTableConfig},
        utils::Context,
    },
    foreign::{
        sha256_helper::{
            circuits::{assign::Sha256HelperTableChip, Sha256HelperTableConfig},
            SHA256_FOREIGN_TABLE_KEY,
        },
        wasm_input_helper::circuits::{
            assign::WasmInputHelperTableChip, WasmInputHelperTableConfig,
            WASM_INPUT_FOREIGN_TABLE_KEY,
        },
        ForeignTableConfig,
    },
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
use specs::{
    host_function::HostPlugin,
    itable::{OpcodeClass, OpcodeClassPlain},
    CompileTable, ExecutionTable,
};
use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read},
    marker::PhantomData,
    path::PathBuf,
};

pub mod config;
pub mod etable_compact;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable_compact;
pub mod rtable;
pub mod utils;

pub(crate) trait FromBn {
    fn zero() -> Self;
    fn from_bn(bn: &BigUint) -> Self;
}

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    rtable: RangeTableConfig<F>,
    itable: InstructionTableConfig<F>,
    imtable: InitMemoryTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    wasm_input_helper_table: WasmInputHelperTableConfig<F>,
    sha256_helper_table: Sha256HelperTableConfig<F>,
}

#[derive(Default, Clone)]
pub struct TestCircuit<F: FieldExt> {
    pub compile_tables: CompileTable,
    pub execution_tables: ExecutionTable,
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
        TestCircuit {
            compile_tables: self.compile_tables.clone(),
            execution_tables: ExecutionTable::default(),
            _data: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let opcode_set = BTreeSet::from([
            OpcodeClassPlain(OpcodeClass::Br as usize),
            OpcodeClassPlain(OpcodeClass::BrIfEqz as usize),
            OpcodeClassPlain(OpcodeClass::Return as usize),
            OpcodeClassPlain(OpcodeClass::Drop as usize),
            OpcodeClassPlain(OpcodeClass::Call as usize),
            OpcodeClassPlain(OpcodeClass::Const as usize),
            OpcodeClassPlain(OpcodeClass::LocalGet as usize),
            OpcodeClassPlain(OpcodeClass::LocalSet as usize),
            OpcodeClassPlain(OpcodeClass::LocalTee as usize),
            OpcodeClassPlain(OpcodeClass::GlobalGet as usize),
            OpcodeClassPlain(OpcodeClass::GlobalSet as usize),
            OpcodeClassPlain(OpcodeClass::Bin as usize),
            OpcodeClassPlain(OpcodeClass::BinBit as usize),
            OpcodeClassPlain(OpcodeClass::BinShift as usize),
            OpcodeClassPlain(OpcodeClass::BrIf as usize),
            OpcodeClassPlain(OpcodeClass::Load as usize),
            OpcodeClassPlain(OpcodeClass::Store as usize),
            OpcodeClassPlain(OpcodeClass::Rel as usize),
            OpcodeClassPlain(OpcodeClass::Select as usize),
            OpcodeClassPlain(OpcodeClass::Test as usize),
            OpcodeClassPlain(OpcodeClass::Conversion as usize),
            OpcodeClassPlain(
                OpcodeClass::ForeignPluginStart as usize + HostPlugin::HostInput as usize,
            ),
            OpcodeClassPlain(
                OpcodeClass::ForeignPluginStart as usize + HostPlugin::Sha256 as usize,
            ),
        ]);

        let constants = meta.fixed_column();
        meta.enable_constant(constants);
        meta.enable_equality(constants);

        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();

        let rtable = RangeTableConfig::configure([0; 7].map(|_| meta.lookup_table_column()));
        let itable = InstructionTableConfig::configure(meta.lookup_table_column());
        let imtable = InitMemoryTableConfig::configure(
            [0; IMTABLE_COLOMNS].map(|_| meta.lookup_table_column()),
        );
        let mtable = MemoryTableConfig::configure(meta, &mut cols, &rtable, &imtable);
        let jtable = JumpTableConfig::configure(meta, &mut cols, &rtable);

        let wasm_input_helper_table = WasmInputHelperTableConfig::configure(meta, &rtable);
        let sha256_helper_table = Sha256HelperTableConfig::configure(meta, &rtable);

        let mut foreign_tables = BTreeMap::<&'static str, Box<dyn ForeignTableConfig<_>>>::new();
        foreign_tables.insert(
            WASM_INPUT_FOREIGN_TABLE_KEY,
            Box::new(wasm_input_helper_table.clone()),
        );
        foreign_tables.insert(
            SHA256_FOREIGN_TABLE_KEY,
            Box::new(sha256_helper_table.clone()),
        );

        let etable = EventTableConfig::configure(
            meta,
            &mut cols,
            &rtable,
            &itable,
            &mtable,
            &jtable,
            &foreign_tables,
            &opcode_set,
        );

        Self::Config {
            rtable,
            itable,
            imtable,
            mtable,
            jtable,
            etable,
            wasm_input_helper_table,
            sha256_helper_table,
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
        let echip = EventTableChip::new(config.etable);
        let wasm_input_chip = WasmInputHelperTableChip::new(config.wasm_input_helper_table);
        let sha256chip = Sha256HelperTableChip::new(config.sha256_helper_table);

        rchip.init(&mut layouter)?;
        wasm_input_chip.init(&mut layouter)?;
        sha256chip.init(&mut layouter)?;

        sha256chip.assign(
            &mut layouter,
            &self
                .execution_tables
                .etable
                .filter_foreign_entries(HostPlugin::Sha256),
        )?;
        wasm_input_chip.assign(
            &mut layouter,
            &self
                .execution_tables
                .etable
                .filter_foreign_entries(HostPlugin::HostInput),
        )?;

        ichip.assign(&mut layouter, &self.compile_tables.itable)?;
        if self.compile_tables.imtable.0.len() > 0 {
            imchip.assign(&mut layouter, &self.compile_tables.imtable)?;
        }

        layouter.assign_region(
            || "jtable mtable etable",
            |region| {
                let mut ctx = Context::new(region);

                let (rest_mops_cell, rest_jops_cell) =
                    { echip.assign(&mut ctx, &self.execution_tables.etable)? };

                ctx.reset();
                mchip.assign(&mut ctx, &self.execution_tables.mtable, rest_mops_cell)?;

                ctx.reset();
                jchip.assign(&mut ctx, &self.execution_tables.jtable, rest_jops_cell)?;

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

impl ZkWasmCircuitBuilder {
    pub fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
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
            let timer = start_timer!(|| format!("build params with K = {}", zkwasm_k()));
            let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(zkwasm_k());
            end_timer!(timer);

            let mut fd = File::create(path.as_path()).unwrap();
            params.write(&mut fd).unwrap();

            params
        }
    }

    fn create_params(&self) -> Params<G1Affine> {
        // Initialize the polynomial commitment parameters
        let timer = start_timer!(|| format!("build params with K = {}", zkwasm_k()));
        let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(zkwasm_k());
        end_timer!(timer);

        params
    }

    fn prepare_vk(
        &self,
        circuit: &TestCircuit<Fr>,
        params: &Params<G1Affine>,
    ) -> VerifyingKey<G1Affine> {
        let timer = start_timer!(|| "build vk");
        let vk = keygen_vk(params, circuit).expect("keygen_vk should not fail");
        end_timer!(timer);

        vk
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
        public_inputs: &Vec<Fr>,
    ) -> Vec<u8> {
        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);

        let timer = start_timer!(|| "create proof");
        create_proof(
            params,
            pk,
            circuits,
            &[&[public_inputs]],
            OsRng,
            &mut transcript,
        )
        .expect("proof generation should not fail");
        end_timer!(timer);

        let proof = transcript.finalize();

        proof
    }

    fn verify_check(
        &self,
        vk: &VerifyingKey<G1Affine>,
        params: &Params<G1Affine>,
        proof: &Vec<u8>,
        public_inputs: &Vec<Fr>,
    ) {
        let public_inputs_size = public_inputs.len();

        let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();

        let strategy = SingleVerifier::new(&params_verifier);
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

        let timer = start_timer!(|| "verify proof");
        verify_proof(
            &params_verifier,
            vk,
            strategy,
            &[&[public_inputs]],
            &mut transcript,
        )
        .unwrap();
        end_timer!(timer);
    }

    pub fn bench(&self, public_inputs: Vec<Fr>) {
        let circuit: TestCircuit<Fr> = self.build_circuit::<Fr>();

        let params = self.prepare_param();

        let vk = self.prepare_vk(&circuit, &params);
        let pk = self.prepare_pk(&circuit, &params, vk);

        let proof = self.create_proof(&[circuit], &params, &pk, &public_inputs);

        self.verify_check(pk.get_vk(), &params, &proof, &public_inputs);
    }

    pub fn bench_with_result(&self, public_inputs: Vec<Fr>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let circuit: TestCircuit<Fr> = self.build_circuit::<Fr>();

        let mut params_buffer: Vec<u8> = vec![];
        let params = self.create_params();
        params.write::<Vec<u8>>(params_buffer.borrow_mut()).unwrap();
        let vk = self.prepare_vk(&circuit, &params);

        let mut vk_buffer: Vec<u8> = vec![];
        vk.write::<Vec<u8>>(vk_buffer.borrow_mut()).unwrap();
        let pk = self.prepare_pk(&circuit, &params, vk);

        let proof = self.create_proof(&[circuit], &params, &pk, &public_inputs);
        self.verify_check(pk.get_vk(), &params, &proof, &public_inputs);

        (params_buffer, vk_buffer, proof)
    }
}
