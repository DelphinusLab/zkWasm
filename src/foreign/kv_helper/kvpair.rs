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

const MERKLE_TREE_HEIGHT:usize = 20;

pub struct KVPairContext {
    pub set_root: Reduce<Fr>,
    pub get_root: Reduce<Fr>,
    pub address: Reduce<Fr>,
    pub set: Reduce<Fr>,
    pub get: Reduce<Fr>,
    pub mongo_merkle: Option<kvpairhelper::MongoMerkle<MERKLE_TREE_HEIGHT>>,
}

fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce {
        cursor: 0,
        rules
    }
}

impl KVPairContext {
    pub fn default() -> Self {
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

    pub fn kvpair_setroot(&mut self, v: u64) {
        self.set_root.reduce(v);
        if self.set_root.cursor == 0 {
            println!("set root: {:?}", &self.set_root.rules[0].bytes_value());
            self.mongo_merkle = Some(
                kvpairhelper::MongoMerkle::construct(
                    [0;32],
                    self.set_root.rules[0].bytes_value()
                        .unwrap()
                        .try_into()
                        .unwrap()
                )
            );
        }
    }

    pub fn kvpair_getroot(&mut self) -> u64 {
        let mt = self.mongo_merkle.as_ref().expect("merkle db not initialized");
        let hash = mt.get_root_hash();
        let values = hash.chunks(8).into_iter().map(|x| {
            u64::from_le_bytes(x.to_vec().try_into().unwrap())
        }).collect::<Vec<u64>>();
        let cursor = self.get_root.cursor;
        self.get_root.reduce(values[self.get_root.cursor]);
        values[cursor]
    }

    pub fn kvpair_address(&mut self, v: u64) {
        self.address.reduce(v);
    }

    pub fn kvpair_set(&mut self, v: u64) {
        self.set.reduce(v);
        if self.set.cursor == 0 {
            let address = self.address.rules[0].u64_value().unwrap() as u32;
            let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
            let mt = self.mongo_merkle.as_mut().expect("merkle db not initialized");
            mt.update_leaf_data_with_proof(
                index,
                &self.set.rules[0].bytes_value().unwrap()
            ).expect("Unexpected failure: update leaf with proof fail");
        }
    }

    pub fn kvpair_get(&mut self) -> u64 {
        let address = self.address.rules[0].u64_value().unwrap() as u32;
        let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
        let mt = self.mongo_merkle.as_ref().expect("merkle db not initialized");
        let (leaf, _) = mt.get_leaf_with_proof(index)
            .expect("Unexpected failure: get leaf fail");
        let cursor = self.get.cursor;
        let values = leaf.data_as_u64();
        self.get.reduce(values[self.get.cursor]);

        values[cursor]
    }
}

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
                context.kvpair_setroot(args.nth(0));
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
                Some(wasmi::RuntimeValue::I64(context.kvpair_getroot() as i64))
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
                context.kvpair_address(args.nth(0));
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
                context.kvpair_set(args.nth(0));
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
                let ret = Some(wasmi::RuntimeValue::I64(context.kvpair_get() as i64));
                ret
            },
        ),
    );
}
