use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use zkwasm_host_circuits::host::merkle::MerkleTree;
use zkwasm_host_circuits::host::kvpair as kvpairhelper;

use super::super::ForeignInst;

#[derive(Default)]
struct KVPairContext {
    pub address_limbs: Vec<u64>,
    pub value_limbs: Vec<u64>,
    pub result_limbs: Vec<u64>,
    pub input_cursor: usize,
    pub result_cursor: usize,
}

const ADDRESS_LIMBNB:usize = 2 + 1; //4 for db id and 1 for address
const VALUE_LIMBNB:usize = 4;
const MERKLE_TREE_HEIGHT:usize = 20;

impl KVPairContext {
}

impl ForeignContext for KVPairContext {}

fn get_merkle_db_address(address_limbs: &Vec<u64>) -> ([u8; 32], u64) {
    let (id, address) = address_limbs.split_at(2);
    let mut id = id.iter().fold(vec![], |acc:Vec<u8>, x| {
        let mut v = acc.clone();
        let mut bytes: Vec<u8> = x.to_le_bytes().to_vec();
        v.append(&mut bytes);
        v
    });
    id.append(&mut [0u8;16].to_vec());
    (id.try_into().unwrap(), address[0])
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_kvpair_foreign(env: &mut HostEnv) {
    let foreign_kvpair_plugin = env
            .external_env
            .register_plugin("foreign_kvpair", Box::new(KVPairContext::default()));

    env.external_env.register_function(
        "kvpair_addr",
        ForeignInst::KVPairAddr as usize,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < ADDRESS_LIMBNB {
                    context.address_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                }
                if context.input_cursor == ADDRESS_LIMBNB {
                    context.input_cursor = 0;
                }
                None
            },
        ),
    );

    env.external_env.register_function(
        "kvpair_set",
        ForeignInst::KVPairSet as usize,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < VALUE_LIMBNB {
                    context.value_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                }
                if context.input_cursor == VALUE_LIMBNB {
                    let (id, address) = get_merkle_db_address(&context.address_limbs);
                    let mut kv = kvpairhelper::MongoMerkle::construct(id, kvpairhelper::MongoMerkle::default_root_hash());
                    let bytes = context.value_limbs.iter().fold(vec![], |acc:Vec<u8>, x| {
                        let mut v = acc.clone();
                        let mut bytes: Vec<u8> = x.to_le_bytes().to_vec();
                        //bytes.resize(8, 0);
                        v.append(&mut bytes);
                        v
                    });
                    let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
                    kv.update_leaf_data_with_proof(index, &bytes)
                        .expect("Unexpected failure: update leaf with proof fail");
                    context.input_cursor = 0;
                    context.value_limbs = vec![];
                }
                None
            },
        ),
    );


    env.external_env.register_function(
        "kvpair_get",
        ForeignInst::KVPairGet as usize,
        ExternalHostCallSignature::Return,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.result_cursor == 0 {
                    let (id, address) = get_merkle_db_address(&context.address_limbs);
                    let kv = kvpairhelper::MongoMerkle::construct(id, kvpairhelper::MongoMerkle::default_root_hash());
                    let index = (address as u32) + (1u32<<MERKLE_TREE_HEIGHT) - 1;
                    let leaf = kv.get_leaf(index)
                        .expect("Unexpected failure: get leaf fail");
                    context.result_limbs = leaf.data_as_u64().to_vec();
                    context.input_cursor = 0;
                }
                let ret = Some(wasmi::RuntimeValue::I64(context.result_limbs[context.result_cursor] as i64));
                context.result_cursor += 1;
                // Change the cursor to 0 if a full value buffer has been read
                if context.result_cursor == VALUE_LIMBNB {
                    context.result_cursor = 0;
                }
                ret
            },
        ),
    );
}
