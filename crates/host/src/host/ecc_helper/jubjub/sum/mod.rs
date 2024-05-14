use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use std::rc::Rc;

use crate::context::ecc_helper::jubjub::sum::BabyJubjubSumContext;
use specs::external_host_call_table::ExternalHostCallSignature;
use zkwasm_host_circuits::circuits::babyjub::AltJubChip;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumNew;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumPush;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumResult;

impl ForeignContext for BabyJubjubSumContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: AltJubChip::max_rounds(self.k as usize),
        })
    }
}

pub fn register_babyjubjubsum_foreign(env: &mut HostEnv) {
    let foreign_babyjubjubsum_plugin = env.external_env.register_plugin(
        "foreign_babyjubjubsum",
        Box::new(BabyJubjubSumContext::default(env.k)),
    );

    env.external_env.register_function(
        "babyjubjub_sum_new",
        JubjubSumNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                context.babyjubjub_sum_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );

    env.external_env.register_function(
        "babyjubjub_sum_push",
        JubjubSumPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                context.babyjubjub_sum_push(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "babyjubjub_sum_finalize",
        JubjubSumResult as usize,
        ExternalHostCallSignature::Return,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(
                    context.babyjubjub_sum_finalize() as i64
                ));
                ret
            },
        ),
    );
}
