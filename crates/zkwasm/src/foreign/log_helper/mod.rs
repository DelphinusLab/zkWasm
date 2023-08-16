use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;
use crate::foreign::log_helper::ExternalOutputForeignInst::*;

struct Context;
impl ForeignContext for Context {}

pub enum ExternalOutputForeignInst {
    ExternalOutputPush = std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>() as isize,
    ExternalOutputPop,
    ExternalOutputAddress,
}

pub struct ExternalOutputContext {
    pub output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub current_key: u64,
}
impl ForeignContext for ExternalOutputContext {}

impl ExternalOutputContext {
    pub fn new(output: Rc<RefCell<HashMap<u64, Vec<u64>>>>) -> Self {
        ExternalOutputContext { output, current_key: 0 }
    }

    pub fn default() -> ExternalOutputContext {
        ExternalOutputContext {
            output: Rc::new(RefCell::new(HashMap::new())),
            current_key: 0,
        }
    }

    pub fn address(&mut self, k: u64) {
        self.current_key = k;
    }

    pub fn push(&self, v: u64) {
        let mut output = self.output.borrow_mut();
        if !output.contains_key(&self.current_key) {
            output.insert(self.current_key.clone(), vec![]);
        }
        let target = output.get_mut(&self.current_key).unwrap();
        target.push(v);
    }

    pub fn pop(&self) -> u64 {
        let mut output = self.output.borrow_mut();
        let target = output.get_mut(&self.current_key).expect(&format!("can't get vec at key {}", self.current_key));

        target.pop().expect(&format!("can't pop vec at key {}", self.current_key))
    }
}

pub fn register_log_foreign(env: &mut HostEnv) {
    let foreign_log_plugin = env
        .external_env
        .register_plugin("foreign_print", Box::new(Context));

    let print = Rc::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);

            println!("{}", value);

            None
        },
    );

    env.external_env.register_function(
        "wasm_dbg",
        Log as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        print,
    );
}

pub fn register_external_output_foreign(env: &mut HostEnv, external_output: Rc<RefCell<HashMap<u64, Vec<u64>>>>) {
    let foreign_output_plugin = env
        .external_env
        .register_plugin("foreign_external_output", Box::new(ExternalOutputContext::new(external_output)));

    let push_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();
            let value: u64 = args.nth(0);
            context.push(value);

            log::debug!("external output push: {}", value);

            None
        },
    );

    let pop_output = Rc::new(
        |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let ret = context.pop();

            log::debug!("external output pop: {}", ret);

            Some(wasmi::RuntimeValue::I64(ret as i64))
        },
    );

    let address_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let value: u64 = args.nth(0);
            context.address(value);

            log::debug!("external output address: {}", value);

            None
        },
    );

    env.external_env.register_function(
        "wasm_external_output_push",
        ExternalOutputPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        push_output,
    );

    env.external_env.register_function(
        "wasm_external_output_pop",
        ExternalOutputPop as usize,
        ExternalHostCallSignature::Return,
        foreign_output_plugin.clone(),
        pop_output,
    );

    env.external_env.register_function(
        "wasm_external_output_address",
        ExternalOutputAddress as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        address_output,
    );
}