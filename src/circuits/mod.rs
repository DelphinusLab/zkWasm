use self::{
    config::{IMTABLE_COLOMNS, VAR_COLUMNS},
    etable_compact::{EventTableChip, EventTableConfig},
    jtable::{JumpTableChip, JumpTableConfig},
    mtable_compact::{MemoryTableChip, MemoryTableConfig},
};
use crate::{
    circuits::{
        config::K,
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
    runtime::WasmRuntime,
};
use ark_std::{end_timer, start_timer};
use halo2_proofs::{
    arithmetic::{CurveAffine, FieldExt, MultiMillerLoop},
    circuit::{Layouter, SimpleFloorPlanner},
    pairing::bn256::{Bn256, Fr, G1Affine},
    plonk::{
        create_proof, keygen_pk, keygen_vk, verify_proof, Circuit, ConstraintSystem, Error,
        Expression, ProvingKey, SingleVerifier, VerifyingKey, VirtualCells,
    },
    poly::commitment::{Params, ParamsVerifier},
    transcript::{
        Blake2bRead, Blake2bWrite, Challenge255, EncodedChallenge, TranscriptRead, TranscriptWrite,
    },
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

    pub fn from_wasm_runtime(wasm_runtime: &impl WasmRuntime) -> Self {
        Self::new(
            wasm_runtime.compile_table(),
            wasm_runtime.execution_tables(),
        )
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
            &imtable,
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

pub struct ZkWasmCircuitBuilder<C: CurveAffine, E: MultiMillerLoop> {
    circuit: TestCircuit<C::ScalarExt>,
    _mark_c: PhantomData<C>,
    _mark_e: PhantomData<E>,
}

const PARAMS: &str = "param.data";

impl<C: CurveAffine, E: MultiMillerLoop<G1Affine = C, Scalar = C::ScalarExt>>
    ZkWasmCircuitBuilder<C, E>
{
    pub fn new(
        compile_tables: CompileTable,
        execution_tables: ExecutionTable,
    ) -> ZkWasmCircuitBuilder<C, E> {
        ZkWasmCircuitBuilder::<C, E> {
            circuit: TestCircuit::new(compile_tables, execution_tables),
            _mark_c: PhantomData,
            _mark_e: PhantomData,
        }
    }

    pub fn from_wasm_runtime(wasm_runtime: &impl WasmRuntime) -> ZkWasmCircuitBuilder<C, E> {
        ZkWasmCircuitBuilder::<C, E>::new(
            wasm_runtime.compile_table(),
            wasm_runtime.execution_tables(),
        )
    }

    fn prepare_param(&self, cache: bool) -> Params<C> {
        let path = PathBuf::from(PARAMS);

        if cache && path.exists() {
            let mut fd = File::open(path.as_path()).unwrap();
            let mut buf = vec![];

            fd.read_to_end(&mut buf).unwrap();
            Params::<C>::read(Cursor::new(buf)).unwrap()
        } else {
            // Initialize the polynomial commitment parameters
            let timer = start_timer!(|| format!("build params with K = {}", K));
            let params: Params<C> = Params::<C>::unsafe_setup::<E>(K);
            end_timer!(timer);

            if cache {
                let mut fd = File::create(path.as_path()).unwrap();
                params.write(&mut fd).unwrap();
            }

            params
        }
    }

    fn prepare_vk(&self, params: &Params<C>) -> VerifyingKey<C> {
        let timer = start_timer!(|| "build vk");
        let vk = keygen_vk(params, &self.circuit).expect("keygen_vk should not fail");
        end_timer!(timer);

        vk
    }

    fn prepare_pk(&self, params: &Params<C>, vk: VerifyingKey<C>) -> ProvingKey<C> {
        let timer = start_timer!(|| "build pk");
        let pk = keygen_pk(&params, vk, &self.circuit).expect("keygen_pk should not fail");
        end_timer!(timer);
        pk
    }

    fn create_proof<Encode: EncodedChallenge<C>, T: TranscriptWrite<C, Encode>>(
        &self,
        params: &Params<C>,
        pk: &ProvingKey<C>,
        public_inputs: &Vec<C::ScalarExt>,
        transcript: &mut T,
    ) {
        let timer = start_timer!(|| "create proof");
        create_proof(
            params,
            pk,
            &vec![self.circuit.clone()],
            &[&[public_inputs]],
            OsRng,
            transcript,
        )
        .expect("proof generation should not fail");
        end_timer!(timer);
    }

    fn verify_check<Encode: EncodedChallenge<C>, T: TranscriptRead<C, Encode>>(
        &self,
        vk: &VerifyingKey<C>,
        params: &Params<C>,
        public_inputs: &Vec<C::ScalarExt>,
        transcript: &mut T,
    ) {
        let public_inputs_size = public_inputs.len();

        let params_verifier: ParamsVerifier<E> = params.verifier(public_inputs_size).unwrap();

        let strategy = SingleVerifier::new(&params_verifier);

        let timer = start_timer!(|| "verify proof");
        verify_proof(
            &params_verifier,
            vk,
            strategy,
            &[&[public_inputs]],
            transcript,
        )
        .unwrap();
        end_timer!(timer);
    }
}

impl ZkWasmCircuitBuilder<G1Affine, Bn256> {
    pub fn run(&self, public_inputs: Vec<Fr>, cache_params: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut params_buffer: Vec<u8> = vec![];
        let params = self.prepare_param(cache_params);
        params.write::<Vec<u8>>(params_buffer.borrow_mut()).unwrap();
        let vk = self.prepare_vk(&params);

        let mut vk_buffer: Vec<u8> = vec![];
        vk.write::<Vec<u8>>(vk_buffer.borrow_mut()).unwrap();
        let pk = self.prepare_pk(&params, vk);

        let proof = {
            let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
            self.create_proof(&params, &pk, &public_inputs, &mut transcript);
            transcript.finalize()
        };

        {
            let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
            self.verify_check(pk.get_vk(), &params, &public_inputs, &mut transcript);
        }

        (params_buffer, vk_buffer, proof)
    }

    pub fn bench(&self, public_inputs: Vec<Fr>) {
        let _ = self.run(public_inputs, true);
    }
}
