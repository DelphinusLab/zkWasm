use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use zkwasm_host_circuits::host::merkle::MerkleTree;
use zkwasm_host_circuits::host::{kvpair as kvpairhelper};

#[derive(Default)]
struct KVPairContext {
    pub address_limbs: Vec<u64>,
    pub value_limbs: Vec<u64>,
    pub result_limbs: Vec<u64>,
    pub input_cursor: usize,
    pub result_cursor: usize,
}

const ADDRESS_LIMBNB:usize = 1;
const VALUE_LIMBNB:usize = 1;
const KVPAIR_ADDR:usize= 7;
const KVPAIR_SET:usize= 8;
const KVPAIR_GET:usize= 9;


impl KVPairContext {
}

impl ForeignContext for KVPairContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_bn254pair_foreign(env: &mut HostEnv) {
    let foreign_kvpair_plugin = env
            .external_env
            .register_plugin("foreign_kvpair", Box::new(KVPairContext::default()));

    let _kv = kvpairhelper::MongoMerkle::construct([0;32]);

    env.external_env.register_function(
        "kvpair_addr",
        KVPAIR_ADDR,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < ADDRESS_LIMBNB {
                    context.address_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                } else {
                    context.input_cursor = 0;
                }

                None
            },
        ),
    );

    env.external_env.register_function(
        "kvpair_set",
        KVPAIR_SET,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < VALUE_LIMBNB {
                    context.value_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                } else {
                    context.input_cursor = 0;
                }
                None
            },
        ),
    );


    env.external_env.register_function(
        "kv254pair_get",
        KVPAIR_GET,
        ExternalHostCallSignature::Return,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.result_cursor == 0 {
                    //context.bn254_gt_to_limbs(gt);
                }
                let ret = Some(wasmi::RuntimeValue::I64(context.result_limbs[context.result_cursor] as i64));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
