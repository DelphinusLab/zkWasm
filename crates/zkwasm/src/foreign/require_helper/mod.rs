use std::rc::Rc;

use specs::host_function::HostPlugin;
use specs::types::ValueType;
use wasmi::tracer::Observer;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;

pub mod etable_op_configure;

struct Context;
impl ForeignContext for Context {}

pub fn register_require_foreign(env: &mut HostEnv) {
    let require = Rc::new(
        |_observer: &Observer, _context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let cond: u32 = args.nth(0);

            if cond == 0 {
                panic!(
                    "require is not satisfied, which is a \
                false assertion in the wasm code. Please check \
                the logic of your image or input."
                )
            }

            None
        },
    );

    env.internal_env
        .register_plugin("require plugin", HostPlugin::Require, Box::new(Context));

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
