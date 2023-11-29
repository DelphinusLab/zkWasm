use std::cell::RefCell;
use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;
use wasmi::RuntimeValue;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;

struct Context {
    is_in_phantom: Rc<RefCell<bool>>,
}
impl ForeignContext for Context {}

pub fn register_phantom_helper_foreign(env: &mut HostEnv) {
    let foreign_phantom_helper_plugin = env.external_env.register_plugin(
        "is_in_phantom",
        Box::new(Context {
            is_in_phantom: env.is_in_phantom.clone(),
        }),
    );

    let cb = Rc::new(|context: &mut dyn ForeignContext, _: wasmi::RuntimeArgs| {
        let context = context.downcast_mut::<Context>().unwrap();

        Some(RuntimeValue::I64((*context.is_in_phantom.borrow()) as i64))
    });

    env.external_env.register_function(
        "is_in_phantom",
        0,
        ExternalHostCallSignature::Return,
        foreign_phantom_helper_plugin,
        cb,
    );
}
