use specs::{host_function::HostPlugin, types::ValueType};
use wasmi::{RuntimeArgs, RuntimeValue};

use crate::runtime::host::HostEnv;

fn lsigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let res = x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22);
    Some(RuntimeValue::I32(res as i32))
}

fn lsigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let res = x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25);
    Some(RuntimeValue::I32(res as i32))
}

fn ssigma0(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let res = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
    Some(RuntimeValue::I32(res as i32))
}

fn ssigma1(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let res = x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10);
    Some(RuntimeValue::I32(res as i32))
}

fn ch(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let y: u32 = args.nth(1);
    let z: u32 = args.nth(2);
    let res = z ^ (x & (y ^ z));
    Some(RuntimeValue::I32(res as i32))
}

fn maj(args: RuntimeArgs) -> Option<RuntimeValue> {
    let x: u32 = args.nth(0);
    let y: u32 = args.nth(1);
    let z: u32 = args.nth(2);
    let res = (x & y) ^ (z & (x ^ y));
    Some(RuntimeValue::I32(res as i32))
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
