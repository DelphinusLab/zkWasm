use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use zkwasm_host_circuits::host::merkle::MerkleTree;
use zkwasm_host_circuits::host::{
    kvpair as kvpairhelper,
    Reduce, ReduceRule
};
use zkwasm_host_circuits::host::ForeignInst::{
    KVPairSet,
    KVPairGet,
    KVPairAddress,
    KVPairGetRoot,
    KVPairSetRoot,
};

use halo2_proofs::pairing::bn256::Fr;

struct KVPairContext {
    pub set_root: Reduce<Fr>,
    pub get_root: Reduce<Fr>,
    pub address: Reduce<Fr>,
    pub set: Reduce<Fr>,
    pub get: Reduce<Fr>,
    pub mongo_merkle: Option<kvpairhelper::MongoMerkle>,
}

fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce {
        cursor: 0,
        rules
    }
}

impl KVPairContext {
    fn default() -> Self {
        KVPairContext {
            set_root: new_reduce(vec![
                ReduceRule::Bytes(vec![], 4),
            ]),
            get_root: new_reduce(vec![
                ReduceRule::Bytes(vec![], 4),
            ]),
            address: new_reduce(vec![
                ReduceRule::U64(0),
            ]),
            set: new_reduce(vec![
                ReduceRule::Bytes(vec![], 4),
            ]),
            get: new_reduce(vec![
                ReduceRule::U64(0),
                ReduceRule::U64(0),
                ReduceRule::U64(0),
                ReduceRule::U64(0),
            ]),

            mongo_merkle: None,
        }
    }
}

const MERKLE_TREE_HEIGHT:usize = 20;

impl KVPairContext {}

impl ForeignContext for KVPairContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_kvpair_foreign(env: &mut HostEnv) {
    let foreign_kvpair_plugin = env
            .external_env
            .register_plugin("foreign_kvpair", Box::new(KVPairContext::default()));

    env.external_env.register_function(
        "kvpair_setroot",
        KVPairSetRoot as usize,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                context.set_root.reduce(args.nth(0));
                if context.set_root.cursor == 0 {
                    println!("set root: {:?}", &context.set_root.rules[0].bytes_value());
                    context.mongo_merkle = Some(
                        kvpairhelper::MongoMerkle::construct(
                            [0;32],
                            context.set_root.rules[0].bytes_value()
                            .unwrap()
                            .try_into()
                            .unwrap()
                        )
                    );
                }
                None
            },
        ),
    );

    env.external_env.register_function(
        "kvpair_getroot",
        KVPairGetRoot as usize,
        ExternalHostCallSignature::Return,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                let mt = context.mongo_merkle.as_ref().expect("merkle db not initialized");
                let hash = mt.get_root_hash();
                let values = hash.chunks(8).into_iter().map(|x| {
                    u64::from_le_bytes(x.to_vec().try_into().unwrap())
                }).collect::<Vec<u64>>();
                let cursor = context.get_root.cursor;
                context.get_root.reduce(values[context.get_root.cursor]);
                Some(wasmi::RuntimeValue::I64(values[cursor] as i64))
            },
        ),
    );

    env.external_env.register_function(
        "kvpair_address",
        KVPairAddress as usize,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                context.address.reduce(args.nth(0));
                None
            },
        ),
    );


    env.external_env.register_function(
        "kvpair_set",
        KVPairSet as usize,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                context.set.reduce(args.nth(0));
                if context.set.cursor == 0 {
                    let address = context.address.rules[0].u64_value().unwrap() as u32;
                    let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
                    let mt = context.mongo_merkle.as_mut().expect("merkle db not initialized");
                    mt.update_leaf_data_with_proof(
                        index,
                        &context.set.rules[0].bytes_value().unwrap()
                    ).expect("Unexpected failure: update leaf with proof fail");
                }
                None
            },
        ),
    );


    env.external_env.register_function(
        "kvpair_get",
        KVPairGet as usize,
        ExternalHostCallSignature::Return,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                let address = context.address.rules[0].u64_value().unwrap() as u32;
                let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
                let mt = context.mongo_merkle.as_ref().expect("merkle db not initialized");
                let (leaf, _) = mt.get_leaf_with_proof(index)
                    .expect("Unexpected failure: get leaf fail");
                let cursor = context.get.cursor;
                let values = leaf.data_as_u64();
                context.get.reduce(values[context.get.cursor]);
                let ret = Some(wasmi::RuntimeValue::I64(values[cursor] as i64));
                ret
            },
        ),
    );
}
