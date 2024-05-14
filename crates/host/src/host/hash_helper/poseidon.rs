use crate::context::hash_helper::poseidon::PoseidonContext;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use specs::external_host_call_table::ExternalHostCallSignature;
use std::rc::Rc;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::poseidon::PoseidonChip;
pub use zkwasm_host_circuits::host::poseidon::POSEIDON_HASHER;
use zkwasm_host_circuits::host::ForeignInst::PoseidonFinalize;
use zkwasm_host_circuits::host::ForeignInst::PoseidonNew;
use zkwasm_host_circuits::host::ForeignInst::PoseidonPush;

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

impl ForeignContext for PoseidonContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: PoseidonChip::max_rounds(self.k as usize),
        })
    }
}

pub fn register_poseidon_foreign(env: &mut HostEnv) {
    let foreign_poseidon_plugin = env.external_env.register_plugin(
        "foreign_poseidon",
        Box::new(PoseidonContext::default(env.k)),
    );

    env.external_env.register_function(
        "poseidon_new",
        PoseidonNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_poseidon_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                log::debug!("buf len is {}", context.buf.len());
                context.poseidon_new(args.nth::<u64>(0) as usize);
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
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                context.poseidon_push(args.nth::<u64>(0) as u64);
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
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<PoseidonContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.poseidon_finalize() as i64))
            },
        ),
    );
}
