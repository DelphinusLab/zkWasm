use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;

struct Context;
impl ForeignContext for Context {}

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
        "log",
        Log as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        print,
    );
}
