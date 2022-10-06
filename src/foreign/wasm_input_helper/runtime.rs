use specs::{host_function::HostPlugin, types::ValueType};

use crate::runtime::host::HostEnv;

fn wasm_input(_args: wasmi::RuntimeArgs) -> Option<wasmi::RuntimeValue> {
    Some(wasmi::RuntimeValue::I64(9))
}

pub fn register_wasm_input_foreign(env: &mut HostEnv) {
    env.register_function(
        "wasm_input",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I64),
        },
        wasm_input,
        HostPlugin::HostInput,
    )
    .unwrap();
}
