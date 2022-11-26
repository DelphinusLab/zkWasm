use specs::{host_function::HostPlugin, types::ValueType};

use crate::runtime::host::{ForeignContext, HostEnv};

pub mod etable_op_configure;

struct Context;
impl ForeignContext for Context {}

pub fn register_require_foreign(env: &mut HostEnv) {
    let require = Box::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let cond: u32 = args.nth(0);

            if cond == 0 {
                unreachable!()
            }

            None
        },
    );

    env.register_function(
        "require",
        0,
        Box::new(Context),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: None,
        },
        require,
        HostPlugin::Require,
    )
    .unwrap();
}
