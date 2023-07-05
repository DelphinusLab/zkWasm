use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use halo2_proofs::pairing::bn256::Fr;
use ff::PrimeField;
use poseidon::Poseidon;
use zkwasm_host_circuits::host::poseidon::{
    gen_hasher,
    T, RATE
};

use zkwasm_host_circuits::host::{
    Reduce, ReduceRule
};


use zkwasm_host_circuits::host::ForeignInst::{
    PoseidonNew,
    PoseidonPush,
    PoseidonFinalize,
};

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

struct Generator {
    pub cursor: usize,
    pub values: Vec<u64>,
}

impl Generator {
    fn gen(&mut self) -> u64 {
        let r = self.values[self.cursor];
        self.cursor += 1;
        if self.cursor == 4 {
            self.cursor = 0;
        }
        r
    }
}

fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce {
        cursor: 0,
        rules
    }
}

struct PoseidonContext {
    pub hasher: Option<Poseidon<Fr, T, RATE>>,
    pub generator: Generator,
    pub buf: Vec<Fr>,
    pub fieldreducer:Reduce<Fr>,
}

impl PoseidonContext {
    fn default() -> Self {
        PoseidonContext {
            hasher: None,
            fieldreducer:new_reduce(vec![ReduceRule::Field(Fr::zero(), 64)]),
            buf: vec![],
            generator: Generator {
                cursor: 0,
                values: vec![],
            },
        }
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
                println!("buf len is {}", context.buf.len());
                context.buf = vec![];
                let new = args.nth::<u64>(0) as usize;
                if new != 0 {
                    context.hasher = Some(gen_hasher());
                }
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
                context.fieldreducer.reduce(args.nth::<u64>(0) as u64);
                if context.fieldreducer.cursor == 0 {
                    context.buf.push(context.fieldreducer.rules[0].field_value().unwrap())
                }
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
                assert!(context.buf.len() == 8);
                if context.generator.cursor == 0 {
                    context.hasher.as_ref().map(|s| {
                        println!("perform hash with {:?}", context.buf);
                        let r = s.clone().update_exact(&context.buf.clone().try_into().unwrap());
                        let dwords:Vec<u8> = r.to_repr().to_vec();
                        context.generator.values = dwords.chunks(8).map(|x| {
                            u64::from_le_bytes(x.to_vec().try_into().unwrap())
                        }).collect::<Vec<u64>>();
                    });
                }
                Some(wasmi::RuntimeValue::I64(context.generator.gen() as i64))
            },
        ),
    );
}
