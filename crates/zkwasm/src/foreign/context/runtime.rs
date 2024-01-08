use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use specs::host_function::HostPlugin;
use specs::host_function::Signature;
use specs::types::ValueType;
use wasmi::tracer::Observer;
use wasmi::RuntimeArgs;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;

use super::Op;

struct Context {
    inputs: Vec<u64>,
    outputs: Arc<Mutex<Vec<u64>>>,
}

impl Context {
    fn new(context_input: Vec<u64>, context_output: Arc<Mutex<Vec<u64>>>) -> Self {
        let mut inputs = context_input.clone();
        inputs.reverse();

        Context {
            inputs,
            outputs: context_output,
        }
    }

    pub fn write_context(&mut self, value: u64) {
        self.outputs.lock().unwrap().push(value)
    }

    pub fn read_context(&mut self) -> u64 {
        self.inputs
            .pop()
            .expect("Failed to pop value from context_in array, please check you inputs")
    }
}

impl ForeignContext for Context {}

pub fn register_context_foreign(
    env: &mut HostEnv,
    context_input: Vec<u64>,
    context_output: Arc<Mutex<Vec<u64>>>,
) {
    env.internal_env.register_plugin(
        "context plugin",
        HostPlugin::Context,
        Box::new(Context::new(context_input, context_output)),
    );

    env.internal_env.register_function(
        "wasm_read_context",
        Signature {
            params: vec![],
            return_type: Some(ValueType::I64),
        },
        HostPlugin::Context,
        Op::ReadContext as usize,
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: RuntimeArgs| {
                let context = context.downcast_mut::<Context>().unwrap();

                Some(wasmi::RuntimeValue::I64(context.read_context() as i64))
            },
        ),
    );

    env.internal_env.register_function(
        "wasm_write_context",
        Signature {
            params: vec![ValueType::I64],
            return_type: None,
        },
        HostPlugin::Context,
        Op::WriteContext as usize,
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: RuntimeArgs| {
                let context = context.downcast_mut::<Context>().unwrap();

                let value: i64 = args.nth(0);
                context.write_context(value as u64);

                None
            },
        ),
    );
}
