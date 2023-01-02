use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::{host_env::HostEnv, ForeignContext};

struct Context;
impl ForeignContext for Context {}

pub fn register_print_foreign(env: &mut HostEnv) {
    let foreign_print_plugin = env
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
        "print",
        0,
        ExternalHostCallSignature::Argument,
        foreign_print_plugin,
        print,
    );
}
