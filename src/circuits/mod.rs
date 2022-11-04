use self::{
    config::{
        ETABLE_END_OFFSET, ETABLE_START_OFFSET, IMTABLE_COLOMNS, JTABLE_START_OFFSET,
        MTABLE_END_OFFSET, MTABLE_START_OFFSET,
    },
    etable_compact::{EventTableChip, EventTableConfig},
    jtable::{JumpTableChip, JumpTableConfig},
    mtable_compact::{MemoryTableChip, MemoryTableConfig},
    shared_column_pool::{SharedColumnChip, SharedColumnPool},
};
use crate::{
    circuits::{
        config::{JTABLE_END_OFFSET, K},
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
use static_assertions::const_assert;
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read, Write},
    marker::PhantomData,
    path::PathBuf,
    rc::Rc,
};

pub mod config;
pub mod etable_compact;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable_compact;
pub mod rtable;
pub mod shared_column_pool;
pub mod utils;

pub(crate) trait FromBn {
    fn zero() -> Self;
    fn from_bn(bn: &BigUint) -> Self;
}

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    shared_column: SharedColumnPool<F>,
    rtable: RangeTableConfig<F>,
    itable: InstructionTableConfig<F>,
    imtable: InitMemoryTableConfig<F>,
    mtable: MemoryTableConfig<F>,
    jtable: JumpTableConfig<F>,
    etable: EventTableConfig<F>,
    wasm_input_helper_table: WasmInputHelperTableConfig<F>,
    sha256_helper_table: Sha256HelperTableConfig<F>,
}

#[derive(Default)]
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
        Self::default()
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

        let rtable = RangeTableConfig::configure([0; 3].map(|_| meta.lookup_table_column()));
        let itable = InstructionTableConfig::configure(meta.lookup_table_column());
        let imtable = InitMemoryTableConfig::configure(
            [0; IMTABLE_COLOMNS].map(|_| meta.lookup_table_column()),
        );

        let shared_column_pool = SharedColumnPool::configure(meta, &rtable);

        let mtable = MemoryTableConfig::configure(meta, &shared_column_pool, &rtable, &imtable);
        let jtable = JumpTableConfig::configure(meta, &shared_column_pool, &rtable);

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
            &shared_column_pool,
            &rtable,
            &itable,
            &mtable,
            &jtable,
            &foreign_tables,
            &opcode_set,
        );

        Self::Config {
            shared_column: shared_column_pool,
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
        let shared_column_chip = SharedColumnChip::new(config.shared_column);

        rchip.init(&mut layouter)?;
        wasm_input_chip.init(&mut layouter)?;
        sha256chip.init(&mut layouter)?;
        shared_column_chip.init(&mut layouter)?;

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
                const_assert!(ETABLE_END_OFFSET <= MTABLE_START_OFFSET);
                const_assert!(MTABLE_END_OFFSET <= JTABLE_START_OFFSET);

                let region = Rc::new(RefCell::new(region));

                let (rest_mops_cell, rest_jops_cell) = {
                    let mut ctx =
                        Context::new(region.clone(), ETABLE_START_OFFSET, ETABLE_END_OFFSET);

                    echip.assign(&mut ctx, &self.execution_tables.etable)?
                };

                {
                    let mut ctx =
                        Context::new(region.clone(), MTABLE_START_OFFSET, MTABLE_END_OFFSET);

                    mchip.assign(&mut ctx, &self.execution_tables.mtable, rest_mops_cell)?;
                }

                {
                    let mut ctx = Context::new(region, JTABLE_START_OFFSET, JTABLE_END_OFFSET);

                    jchip.assign(&mut ctx, &self.execution_tables.jtable, rest_jops_cell)?;
                }

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
    pub fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
        TestCircuit::new(self.compile_tables.clone(), self.execution_tables.clone())
    }

    pub fn prepare_param(&self) -> Params<G1Affine> {
        let path = PathBuf::from(PARAMS);

        if path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();
            Params::<G1Affine>::read(Cursor::new(buf)).unwrap()
        } else {
            // Initialize the polynomial commitment parameters
            let timer = start_timer!(|| format!("build params with K = {}", K));
            let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(K);
            end_timer!(timer);

            let mut fd = File::create(path.as_path()).unwrap();
            params.write(&mut fd).unwrap();

            params
        }
    }

    fn create_params(&self) -> Params<G1Affine> {
        // Initialize the polynomial commitment parameters
        let timer = start_timer!(|| format!("build params with K = {}", K));
        let params: Params<G1Affine> = Params::<G1Affine>::unsafe_setup::<Bn256>(K);
        end_timer!(timer);

        params
    }

    pub fn prepare_vk(
        &self,
        circuit: &TestCircuit<Fr>,
        params: &Params<G1Affine>,
    ) -> VerifyingKey<G1Affine> {
        let path = PathBuf::from(VK);

        let vk = if path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();

            VerifyingKey::<G1Affine>::read::<_, TestCircuit<_>>(&mut Cursor::new(&buf), params)
                .unwrap()
        } else {
            let timer = start_timer!(|| "build vk");
            let vk = keygen_vk(params, circuit).expect("keygen_vk should not fail");
            end_timer!(timer);

            let mut fd = File::create(path.as_path()).unwrap();
            vk.write(&mut fd).unwrap();

            vk
        };

        println!("instance commitments: {}", vk.cs.num_instance_columns);
        println!("advice commitments: {}", vk.cs.num_advice_columns);
        println!("fixed commitments: {}", vk.fixed_commitments.len());
        println!("lookup argument * 3: {}", vk.cs.lookups.len() * 3);
        println!("permutation argument: {}", vk.permutation.commitments.len());
        println!("degree: {}", vk.cs.degree() - 1);

        println!(
            "total: {}",
            vk.cs.num_instance_columns
                + vk.cs.num_advice_columns
                + vk.fixed_commitments.len()
                + vk.cs.lookups.len() * 3
                + vk.permutation.commitments.len()
                + vk.cs.degree()
                - 1
        );

        vk
    }

    pub fn prepare_pk(
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

    pub fn create_proof(
        &self,
        circuits: &[TestCircuit<Fr>],
        params: &Params<G1Affine>,
        pk: &ProvingKey<G1Affine>,
        public_inputs: &Vec<Fr>,
    ) -> Vec<u8> {
        let path = PathBuf::from(PROOF);

        if path.exists() {
            let mut buf = vec![];
            let mut fd = std::fs::File::open(path).unwrap();
            fd.read_to_end(&mut buf).unwrap();
            buf
        } else {
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

            let mut fd = std::fs::File::create(path).unwrap();
            fd.write(&proof).unwrap();

            proof
        }
    }

    pub fn verify_check(
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
