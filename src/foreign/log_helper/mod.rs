use specs::{host_function::HostPlugin, types::ValueType};

use crate::runtime::host::{ForeignContext, HostEnv};

pub mod etable_op_configure;

struct Context;
impl ForeignContext for Context {}

pub fn register_log_foreign(env: &mut HostEnv) {
    let log = Box::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u32 = args.nth(0);

            print!("{}", value);

            None
        },
    );

    env.register_function(
        "log",
        0,
        Box::new(Context),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: None,
        },
        log,
        HostPlugin::Log,
    )
    .unwrap();
}
