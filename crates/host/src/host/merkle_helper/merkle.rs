use crate::context::merkle_helper::merkle::MerkleContext;
use crate::context::merkle_helper::merkle::MERKLE_TREE_HEIGHT;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use halo2_proofs::pairing::bn256::Fr;
use specs::external_host_call_table::ExternalHostCallSignature;
use std::cell::RefCell;
use std::rc::Rc;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::merkle::MerkleChip;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::ForeignInst::MerkleAddress;
use zkwasm_host_circuits::host::ForeignInst::MerkleGet;
use zkwasm_host_circuits::host::ForeignInst::MerkleGetRoot;
use zkwasm_host_circuits::host::ForeignInst::MerkleSet;
use zkwasm_host_circuits::host::ForeignInst::MerkleSetRoot;

impl ForeignContext for MerkleContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: MerkleChip::<Fr, MERKLE_TREE_HEIGHT>::max_rounds(self.k as usize),
        })
    }
}

pub fn register_merkle_foreign(env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
    let foreign_merkle_plugin = env.external_env.register_plugin(
        "foreign_merkle",
        Box::new(MerkleContext::new(env.k, tree_db)),
    );

    env.external_env.register_function(
        "merkle_setroot",
        MerkleSetRoot as usize,
        ExternalHostCallSignature::Argument,
        foreign_merkle_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<MerkleContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(context.merkle_get() as i64));
                ret
            },
        ),
    );
}
