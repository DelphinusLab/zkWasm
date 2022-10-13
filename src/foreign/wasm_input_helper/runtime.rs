use specs::{host_function::HostPlugin, types::ValueType};

use crate::runtime::host::{ForeignContext, HostEnv};

struct Context {
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
}

impl Context {
    pub fn new(public_inputs: Vec<u64>, private_inputs: Vec<u64>) -> Self {
        Context {
            public_inputs,
            private_inputs,
        }
    }

    pub fn pop_public(&mut self) -> u64 {
        self.public_inputs.remove(0)
    }

    pub fn pop_private(&mut self) -> u64 {
        self.private_inputs.remove(0)
    }
}

impl ForeignContext for Context {}

// TODO: invoke this in WasmRuntime
pub fn register_wasm_input_foreign(
    env: &mut HostEnv,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) {
    let wasm_input = Box::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            let arg: i32 = args.nth(0);
            assert!(arg == 0 || arg == 1);

            let input = if arg == 1 {
                context.pop_public()
            } else {
                context.pop_private()
            };

            Some(wasmi::RuntimeValue::I64(input as i64))
        },
    );

    env.register_function(
        "wasm_input",
        0,
        Box::new(Context::new(public_inputs, private_inputs)),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I64),
        },
        wasm_input,
        HostPlugin::HostInput,
    )
    .unwrap();
}
