use self::{
    brtable::BrTableConfig, external_host_call_table::ExternalHostCallTableConfig,
    jtable::JumpTableConfig, mtable_compact::MemoryTableConfig,
};
use crate::circuits::{
    config::zkwasm_k, itable::InstructionTableConfig, rtable::RangeTableConfig, utils::Context,
};
use ark_std::{end_timer, start_timer};
use halo2_proofs::{
    arithmetic::FieldExt,
    pairing::bn256::{Bn256, Fr, G1Affine},
    plonk::{
        create_proof, keygen_pk, keygen_vk, verify_proof, ConstraintSystem, Expression, ProvingKey,
        SingleVerifier, VerifyingKey, VirtualCells,
    },
    poly::commitment::{Params, ParamsVerifier},
    transcript::{Blake2bRead, Blake2bWrite, Challenge255},
};
use num_bigint::BigUint;
use rand::rngs::OsRng;
use specs::{host_function::HostPlugin, itable::OpcodeClassPlain, Tables};
use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Read},
    marker::PhantomData,
    path::PathBuf,
};

mod bit_table;
pub(crate) mod cell;
pub(crate) mod etable_v2;
mod external_host_call_table;
mod mtable_v2;
mod traits;

pub mod brtable;
pub mod config;
pub mod etable_compact;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable_compact;
pub mod rtable;
pub mod test_circuit;
pub mod utils;

#[derive(Clone)]
pub struct CircuitConfigure {
    pub initial_memory_pages: u32,
    pub maximal_memory_pages: u32,
    pub first_consecutive_zero_memory_offset: u32,
    pub opcode_selector: HashSet<OpcodeClassPlain>,
}

#[thread_local]
static mut CIRCUIT_CONFIGURE: Option<CircuitConfigure> = None;

#[derive(Default, Clone)]
pub struct TestCircuit<F: FieldExt> {
    pub fid_of_entry: u32,
    pub tables: Tables,
    _data: PhantomData<F>,
}

impl<F: FieldExt> TestCircuit<F> {
    pub fn new(fid_of_entry: u32, tables: Tables) -> Self {
        unsafe {
            CIRCUIT_CONFIGURE = Some(CircuitConfigure {
                first_consecutive_zero_memory_offset: tables
                    .compilation_tables
                    .imtable
                    .first_consecutive_zero_memory(),
                initial_memory_pages: tables.compilation_tables.configure_table.init_memory_pages,
                maximal_memory_pages: tables
                    .compilation_tables
                    .configure_table
                    .maximal_memory_pages,
                opcode_selector: tables.compilation_tables.itable.opcode_class(),
            });
        }

        TestCircuit {
            fid_of_entry,
            tables,
            _data: PhantomData,
        }
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
    pub fid_of_entry: u32,
    pub tables: Tables,
}

const PARAMS: &str = "param.data";

impl ZkWasmCircuitBuilder {
    pub fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
        TestCircuit::new(self.fid_of_entry, self.tables.clone())
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

        transcript.finalize()
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
}
