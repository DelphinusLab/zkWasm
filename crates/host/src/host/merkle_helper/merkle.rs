use delphinus_zkwasm::circuits::config::zkwasm_k;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use halo2_proofs::pairing::bn256::Fr;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::tracer::Observer;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::merkle::MerkleChip;
use zkwasm_host_circuits::host::datahash as datahelper;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::merkle::MerkleTree;
use zkwasm_host_circuits::host::mongomerkle as merklehelper;
use zkwasm_host_circuits::host::ForeignInst::MerkleAddress;
use zkwasm_host_circuits::host::ForeignInst::MerkleGet;
use zkwasm_host_circuits::host::ForeignInst::MerkleGetRoot;
use zkwasm_host_circuits::host::ForeignInst::MerkleSet;
use zkwasm_host_circuits::host::ForeignInst::MerkleSetRoot;
use zkwasm_host_circuits::host::Reduce;
use zkwasm_host_circuits::host::ReduceRule;

const MERKLE_TREE_HEIGHT: usize = 32;

pub struct MerkleContext {
    pub set_root: Reduce<Fr>,
    pub get_root: Reduce<Fr>,
    pub address: Reduce<Fr>,
    pub set: Reduce<Fr>,
    pub data: [u64; 4],
    pub data_cursor: usize,
    pub fetch: bool,
    pub mongo_merkle: Option<merklehelper::MongoMerkle<MERKLE_TREE_HEIGHT>>,
    pub mongo_datahash: datahelper::MongoDataHash,
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
    pub used_round: usize,
}

fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce { cursor: 0, rules }
}

impl MerkleContext {
    pub fn new(tree_db: Option<Rc<RefCell<dyn TreeDB>>>) -> Self {
        MerkleContext {
            set_root: new_reduce(vec![ReduceRule::Bytes(vec![], 4)]),
            get_root: new_reduce(vec![ReduceRule::Bytes(vec![], 4)]),
            address: new_reduce(vec![ReduceRule::U64(0)]),
            set: new_reduce(vec![ReduceRule::Bytes(vec![], 4)]),
            fetch: false,
            data: [0; 4],
            data_cursor: 0,
            mongo_merkle: None,
            mongo_datahash: datahelper::MongoDataHash::construct([0; 32], tree_db.clone()),
            tree_db,
            used_round: 0,
        }
    }

    pub fn merkle_setroot(&mut self, v: u64) {
        self.set_root.reduce(v);
        if self.set_root.cursor == 0 {
            log::debug!("set root: {:?}", &self.set_root.rules[0].bytes_value());
            self.mongo_merkle = Some(merklehelper::MongoMerkle::construct(
                [0; 32],
                self.set_root.rules[0]
                    .bytes_value()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                self.tree_db.clone(),
            ));
        }
    }

    pub fn merkle_getroot(&mut self) -> u64 {
        let mt = self
            .mongo_merkle
            .as_ref()
            .expect("merkle db not initialized");
        let hash = mt.get_root_hash();
        let values = hash
            .chunks(8)
            .into_iter()
            .map(|x| u64::from_le_bytes(x.to_vec().try_into().unwrap()))
            .collect::<Vec<u64>>();
        let cursor = self.get_root.cursor;
        self.get_root.reduce(values[self.get_root.cursor]);
        values[cursor]
    }

    /// reset the address of merkle op together with the data and data_cursor
    pub fn merkle_address(&mut self, v: u64) {
        if self.address.cursor == 0 {
            self.used_round += 1;
        }
        self.data = [0; 4];
        self.fetch = false;
        self.data_cursor = 0;
        self.address.reduce(v);
    }

    pub fn merkle_set(&mut self, v: u64) {
        self.set.reduce(v);
        if self.set.cursor == 0 {
            let address = self.address.rules[0].u64_value().unwrap() as u32;
            let index = (address as u64) + (1u64 << MERKLE_TREE_HEIGHT) - 1;
            let mt = self
                .mongo_merkle
                .as_mut()
                .expect("merkle db not initialized");
            let hash = self.set.rules[0].bytes_value().unwrap();
            mt.update_leaf_data_with_proof(index, &hash)
                .expect("Unexpected failure: update leaf with proof fail");
        }
    }

    pub fn merkle_get(&mut self) -> u64 {
        let address = self.address.rules[0].u64_value().unwrap() as u32;
        let index = (address as u64) + (1u64 << MERKLE_TREE_HEIGHT) - 1;
        let mt = self
            .mongo_merkle
            .as_ref()
            .expect("merkle db not initialized");
        let (leaf, _) = mt
            .get_leaf_with_proof(index)
            .expect("Unexpected failure: get leaf fail");
        let values = leaf.data_as_u64();
        if self.data_cursor == 0 {
            self.data = values;
        }
        let v = values[self.data_cursor];
        self.data_cursor += 1;
        return v;
    }
}

impl MerkleContext {}

impl ForeignContext for MerkleContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: MerkleChip::<Fr, MERKLE_TREE_HEIGHT>::max_rounds(zkwasm_k() as usize),
        })
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_merkle_foreign(env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
    let foreign_merkle_plugin = env
        .external_env
        .register_plugin("foreign_merkle", Box::new(MerkleContext::new(tree_db)));

    env.external_env.register_function(
        "merkle_setroot",
        MerkleSetRoot as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                context.merkle_setroot(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "merkle_getroot",
        MerkleGetRoot as usize,
        ExternalHostCallSignature::Return,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.merkle_getroot() as i64))
            },
        ),
    );

    env.external_env.register_function(
        "merkle_address",
        MerkleAddress as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                context.merkle_address(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "merkle_set",
        MerkleSet as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                context.merkle_set(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "merkle_get",
        MerkleGet as usize,
        ExternalHostCallSignature::Return,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(context.merkle_get() as i64));
                ret
            },
        ),
    );
}
