use crate::context::merkle_helper::datacache::CacheContext;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use std::cell::RefCell;
use std::rc::Rc;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::host::ForeignInst::CacheFetchData;
use zkwasm_host_circuits::host::ForeignInst::CacheSetHash;
use zkwasm_host_circuits::host::ForeignInst::CacheSetMode;
use zkwasm_host_circuits::host::ForeignInst::CacheStoreData;

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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<CacheContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(context.fetch_data() as i64));
                ret
            },
        ),
    );
}
