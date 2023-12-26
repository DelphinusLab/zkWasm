use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use halo2_proofs::pairing::bn256::Fr;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::tracer::Observer;
use zkwasm_host_circuits::host::datahash as datahelper;
use zkwasm_host_circuits::host::datahash::DataHashRecord;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::ForeignInst::CacheFetchData;
use zkwasm_host_circuits::host::ForeignInst::CacheSetHash;
use zkwasm_host_circuits::host::ForeignInst::CacheSetMode;
use zkwasm_host_circuits::host::ForeignInst::CacheStoreData;
use zkwasm_host_circuits::host::Reduce;
use zkwasm_host_circuits::host::ReduceRule;

const FETCH_MODE: u64 = 0;
const STORE_MODE: u64 = 1;

pub struct CacheContext {
    pub mode: u64,
    pub hash: Reduce<Fr>,
    pub data: Vec<u64>,
    pub fetch: bool,
    pub mongo_datahash: datahelper::MongoDataHash,
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
}

fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce { cursor: 0, rules }
}

impl CacheContext {
    pub fn new(tree_db: Option<Rc<RefCell<dyn TreeDB>>>) -> Self {
        CacheContext {
            mode: 0,
            hash: new_reduce(vec![ReduceRule::Bytes(vec![], 4)]),
            fetch: false,
            data: vec![],
            mongo_datahash: datahelper::MongoDataHash::construct([0; 32], tree_db.clone()),
            tree_db,
        }
    }

    pub fn set_mode(&mut self, v: u64) {
        self.mode = v;
        self.data = vec![];
    }

    pub fn set_data_hash(&mut self, v: u64) {
        self.hash.reduce(v);
        if self.hash.cursor == 0 {
            let hash: [u8; 32] = self.hash.rules[0]
                .bytes_value()
                .unwrap()
                .try_into()
                .unwrap();
            if self.mode == FETCH_MODE {
                let datahashrecord = self.mongo_datahash.get_record(&hash).unwrap();
                self.data = datahashrecord.map_or(vec![], |r| {
                    r.data
                        .chunks_exact(8)
                        .into_iter()
                        .into_iter()
                        .map(|x| u64::from_le_bytes(x.try_into().unwrap()))
                        .collect::<Vec<u64>>()
                });
                self.fetch = false;
            } else if self.mode == STORE_MODE {
                // put data and hash into mongo_datahash
                if !self.data.is_empty() {
                    self.mongo_datahash
                        .update_record({
                            DataHashRecord {
                                hash,
                                data: self
                                    .data
                                    .iter()
                                    .map(|x| x.to_le_bytes())
                                    .flatten()
                                    .collect::<Vec<u8>>(),
                            }
                        })
                        .unwrap();
                }
            }
        }
    }

    pub fn fetch_data(&mut self) -> u64 {
        if self.fetch == false {
            self.fetch = true;
            self.data.reverse();
            self.data.len() as u64
        } else {
            self.data.pop().unwrap()
        }
    }

    pub fn store_data(&mut self, v: u64) {
        self.data.push(v);
    }
}

impl CacheContext {}

impl ForeignContext for CacheContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        // pure witness function
        None
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_datacache_foreign(env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
    let foreign_merkle_plugin = env
        .external_env
        .register_plugin("foreign_cache", Box::new(CacheContext::new(tree_db)));

    env.external_env.register_function(
        "cache_set_mode",
        CacheSetMode as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<CacheContext>().unwrap();
                context.set_mode(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "cache_set_hash",
        CacheSetHash as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<CacheContext>().unwrap();
                context.set_data_hash(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "cache_store_data",
        CacheStoreData as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<CacheContext>().unwrap();
                context.store_data(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "cache_fetch_data",
        CacheFetchData as usize,
        ExternalHostCallSignature::Return,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<CacheContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(context.fetch_data() as i64));
                ret
            },
        ),
    );
}
