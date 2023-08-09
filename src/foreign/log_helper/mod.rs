use std::cell::RefCell;
use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;

struct Context;
impl ForeignContext for Context {}

struct OutputContext {
    output: Rc<RefCell<Vec<u64>>>,
}
impl ForeignContext for OutputContext {}

impl OutputContext {
    pub fn new(output: Rc<RefCell<Vec<u64>>>) -> Self {
        OutputContext { output }
    }

    pub fn push(&self, v: u64) {
        let mut output = self.output.borrow_mut();
        output.push(v);
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

pub fn register_log_output_foreign(env: &mut HostEnv) {
    let outputs =  env.log_outputs.clone();
    let foreign_output_plugin = env
        .external_env
        .register_plugin("foreign_log_output", Box::new(OutputContext::new(outputs)));

    let get_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<OutputContext>().unwrap();
            let value: u64 = args.nth(0);
            context.push(value);

            log::debug!("get internal output: {}", value);

            None
        },
    );

    env.external_env.register_function(
        "wasm_log_output",
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>(),
        ExternalHostCallSignature::Argument,
        foreign_output_plugin,
        get_output,
    );
}
