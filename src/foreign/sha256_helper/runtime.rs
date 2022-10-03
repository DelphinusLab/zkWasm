use specs::{host_function::HostPlugin, types::ValueType};
use wasmi::{RuntimeArgs, RuntimeValue};

use crate::runtime::host::HostEnv;

fn lsigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    Some(RuntimeValue::I32(
        x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22),
    ))
}

fn lsigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    Some(RuntimeValue::I32(
        x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25),
    ))
}

fn ssigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    Some(RuntimeValue::I32(
        x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3),
    ))
}

fn ssigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    Some(RuntimeValue::I32(
        x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10),
    ))
}

fn ch(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    let y: i32 = args.nth(1);
    let z: i32 = args.nth(2);
    Some(RuntimeValue::I32(z ^ (x & (y ^ z))))
}

fn maj(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: i32 = args.nth(0);
    let y: i32 = args.nth(1);
    let z: i32 = args.nth(2);
    Some(RuntimeValue::I32((x & y) ^ (z & (x ^ y))))
}

pub fn register_sha256_foreign(env: &mut HostEnv) {
    env.register_function(
        "Sigma1",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        lsigma1,
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        "Ch",
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        ch,
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        "Sigma0",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        lsigma0,
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        "Maj",
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        maj,
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        "sigma0",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        ssigma0,
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        "sigma1",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        ssigma1,
        HostPlugin::Sha256,
    )
    .unwrap();
}
