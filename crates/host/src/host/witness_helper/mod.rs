use delphinus_zkwasm::runtime::host::ForeignContext;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmi::tracer::Observer;

use crate::HostEnv;
use zkwasm_host_circuits::host::ForeignInst::WitnessIndexedInsert;
use zkwasm_host_circuits::host::ForeignInst::WitnessIndexedPop;
use zkwasm_host_circuits::host::ForeignInst::WitnessIndexedPush;
use zkwasm_host_circuits::host::ForeignInst::WitnessInsert;
use zkwasm_host_circuits::host::ForeignInst::WitnessPop;
use zkwasm_host_circuits::host::ForeignInst::WitnessSetIndex;
use zkwasm_host_circuits::host::ForeignInst::WitnessTraceSize;

#[derive(Default)]
pub struct WitnessContext {
    pub buf: Vec<u64>,
    pub indexed_buf: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub focus: u64,
}

impl WitnessContext {
    fn new(indexed_map: Rc<RefCell<HashMap<u64, Vec<u64>>>>) -> Self {
        WitnessContext {
            buf: vec![],
            indexed_buf: indexed_map,
            focus: 0,
        }
    }
}

impl WitnessContext {
    pub fn witness_insert(&mut self, new: u64) {
        self.buf.insert(0, new);
    }

    pub fn witness_pop(&mut self) -> u64 {
        self.buf.pop().unwrap()
    }

    pub fn witness_set_index(&mut self, index: u64) {
        self.focus = index;
    }

    pub fn witness_indexed_insert(&mut self, new: u64) {
        let mut bind = self.indexed_buf.borrow_mut();
        let buf = bind.get_mut(&self.focus);
        if let Some(vec) = buf {
            vec.insert(0, new);
        } else {
            bind.insert(self.focus, vec![new]);
        }
    }

    pub fn witness_indexed_push(&mut self, new: u64) {
        let mut bind = self.indexed_buf.borrow_mut();
        let buf = bind.get_mut(&self.focus);
        if let Some(vec) = buf {
            vec.push(new);
        } else {
            bind.insert(self.focus, vec![new]);
        }
    }

    pub fn witness_indexed_pop(&mut self) -> u64 {
        let mut bind = self.indexed_buf.borrow_mut();
        let buf = bind.get_mut(&self.focus).unwrap();
        buf.pop().unwrap()
    }
}

impl ForeignContext for WitnessContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_witness_foreign(env: &mut HostEnv, index_map: Rc<RefCell<HashMap<u64, Vec<u64>>>>) {
    let foreign_witness_plugin = env
        .external_env
        .register_plugin("foreign_witness", Box::new(WitnessContext::new(index_map)));

    env.external_env.register_function(
        "wasm_witness_insert",
        WitnessInsert as usize,
        ExternalHostCallSignature::Argument,
        foreign_witness_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                context.witness_insert(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "wasm_witness_set_index",
        WitnessSetIndex as usize,
        ExternalHostCallSignature::Argument,
        foreign_witness_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                context.witness_set_index(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "wasm_witness_indexed_insert",
        WitnessIndexedInsert as usize,
        ExternalHostCallSignature::Argument,
        foreign_witness_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                context.witness_indexed_insert(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "wasm_witness_indexed_push",
        WitnessIndexedPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_witness_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                context.witness_indexed_push(args.nth::<u64>(0) as u64);
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
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.witness_pop() as i64))
            },
        ),
    );

    env.external_env.register_function(
        "wasm_witness_indexed_pop",
        WitnessIndexedPop as usize,
        ExternalHostCallSignature::Return,
        foreign_witness_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<WitnessContext>().unwrap();
                Some(wasmi::RuntimeValue::I64(
                    context.witness_indexed_pop() as i64
                ))
            },
        ),
    );

    env.external_env.register_function(
        "wasm_trace_size",
        WitnessTraceSize as usize,
        ExternalHostCallSignature::Return,
        foreign_witness_plugin.clone(),
        Rc::new(
            |obs: &Observer, _context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                Some(wasmi::RuntimeValue::I64(obs.counter as i64))
            },
        ),
    );
}
