use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::tracer::Tracer;

use crate::HostEnv;
use zkwasm_host_circuits::host::ForeignInst::WitnessInsert;
use zkwasm_host_circuits::host::ForeignInst::WitnessPop;

#[derive(Default)]
pub struct WitnessContext {
    pub buf: Vec<u64>,
}

impl WitnessContext {
    pub fn witness_insert(&mut self, new: u64) {
        self.buf.insert(0, new);
    }

    pub fn witness_pop(&mut self) -> u64 {
        self.buf.pop().unwrap()
    }
}

impl ForeignContext for WitnessContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        None
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_witness_foreign(env: &mut HostEnv) {
    let foreign_witness_plugin = env
        .external_env
        .register_plugin("foreign_witness", Box::new(WitnessContext::default()));

    env.external_env.register_function(
        "wasm_witness_insert",
        WitnessInsert as usize,
        ExternalHostCallSignature::Argument,
        foreign_witness_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext,
             args: wasmi::RuntimeArgs,
             _tracer: Rc<RefCell<Tracer>>| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                context.witness_insert(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "wasm_witness_pop",
        WitnessPop as usize,
        ExternalHostCallSignature::Return,
        foreign_witness_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext,
             _args: wasmi::RuntimeArgs,
             _tracer: Rc<RefCell<Tracer>>| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.witness_pop() as i64))
            },
        ),
    );
}
