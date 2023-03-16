use std::rc::Rc;

use specs::{host_function::HostPlugin, types::ValueType};

use crate::runtime::host::{host_env::HostEnv, ForeignContext};

#[cfg(not(feature = "v2"))]
pub mod etable_op_configure;
#[cfg(feature = "v2")]
pub mod etable_op_configure;

struct Context;
impl ForeignContext for Context {}

pub fn register_require_foreign(env: &mut HostEnv) {
    let require = Rc::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let cond: u32 = args.nth(0);

            if cond == 0 {
                unreachable!()
            }

            None
        },
    );

    env.internal_env
        .register_plugin(HostPlugin::Require, Box::new(Context));

    env.internal_env.register_function(
        "require",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: None,
        },
        HostPlugin::Require,
        0,
        require,
    );
}
