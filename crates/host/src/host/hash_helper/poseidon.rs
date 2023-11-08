use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use ff::PrimeField;
use halo2_proofs::pairing::bn256::Fr;
use poseidon::Poseidon;
use std::rc::Rc;
pub use zkwasm_host_circuits::host::poseidon::POSEIDON_HASHER;

use zkwasm_host_circuits::host::Reduce;
use zkwasm_host_circuits::host::ReduceRule;

use zkwasm_host_circuits::host::ForeignInst::PoseidonFinalize;
use zkwasm_host_circuits::host::ForeignInst::PoseidonNew;
use zkwasm_host_circuits::host::ForeignInst::PoseidonPush;

/// Foreign functions that supports the following C code library
///
/// void poseidon(uint64_t* data, uint32_t size, uint64_t* r)
/// {
///     int i;
///     poseidon_new(size);
///     for(i=0; i<size; i=++) {
///         uint64_t* a = data[i];
///         poseidon_push(data[i]);
///     }
///     r[0] = poseidon_finalize();
///     r[1] = poseidon_finalize();
///     r[2] = poseidon_finalize();
///     r[3] = poseidon_finalize();
///     wasm_dbg(r[0]);
///     wasm_dbg(r[1]);
///     wasm_dbg(r[2]);
///     wasm_dbg(r[3]);
/// }

pub struct Generator {
    pub cursor: usize,
    pub values: Vec<u64>,
}

impl Generator {
    pub fn gen(&mut self) -> u64 {
        let r = self.values[self.cursor];
        self.cursor += 1;
        if self.cursor == 4 {
            self.cursor = 0;
        }
        r
    }
}

pub fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce { cursor: 0, rules }
}

pub struct PoseidonContext {
    pub hasher: Option<Poseidon<Fr, 9, 8>>,
    pub generator: Generator,
    pub buf: Vec<Fr>,
    pub fieldreducer: Reduce<Fr>,
}

impl PoseidonContext {
    pub fn default() -> Self {
        PoseidonContext {
            hasher: None,
            fieldreducer: new_reduce(vec![ReduceRule::Field(Fr::zero(), 64)]),
            buf: vec![],
            generator: Generator {
                cursor: 0,
                values: vec![],
            },
        }
    }

    pub fn poseidon_new(&mut self, new: usize) {
        self.buf = vec![];
        if new != 0 {
            self.hasher = Some(POSEIDON_HASHER.clone());
        }
    }

    pub fn poseidon_push(&mut self, v: u64) {
        self.fieldreducer.reduce(v);
        if self.fieldreducer.cursor == 0 {
            self.buf
                .push(self.fieldreducer.rules[0].field_value().unwrap())
        }
    }

    pub fn poseidon_finalize(&mut self) -> u64 {
        assert!(self.buf.len() == 8);
        if self.generator.cursor == 0 {
            self.hasher.as_mut().map(|s| {
                log::debug!("perform hash with {:?}", self.buf);
                let r = s.update_exact(&self.buf.clone().try_into().unwrap());
                let dwords: Vec<u8> = r.to_repr().to_vec();
                self.generator.values = dwords
                    .chunks(8)
                    .map(|x| u64::from_le_bytes(x.to_vec().try_into().unwrap()))
                    .collect::<Vec<u64>>();
            });
        }
        self.generator.gen()
    }
}

impl ForeignContext for PoseidonContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_poseidon_foreign(env: &mut HostEnv) {
    let foreign_poseidon_plugin = env
        .external_env
        .register_plugin("foreign_sh256", Box::new(PoseidonContext::default()));

    env.external_env.register_function(
        "poseidon_new",
        PoseidonNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_poseidon_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                log::debug!("buf len is {}", context.buf.len());
                context.poseidon_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );

    env.external_env.register_function(
        "poseidon_push",
        PoseidonPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_poseidon_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                context.poseidon_push(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "poseidon_finalize",
        PoseidonFinalize as usize,
        ExternalHostCallSignature::Return,
        foreign_poseidon_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.poseidon_finalize() as i64))
            },
        ),
    );
}
