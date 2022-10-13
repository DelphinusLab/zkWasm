use specs::{host_function::HostPlugin, types::ValueType};
use wasmi::{RuntimeArgs, RuntimeValue};

use crate::runtime::host::{ForeignContext, HostEnv};

use super::{
    Sha256HelperOp, SHA256_FOREIGN_FUNCTION_NAME_CH, SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0,
    SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1, SHA256_FOREIGN_FUNCTION_NAME_MAJ,
    SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0, SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1,
};

struct Context {}
impl ForeignContext for Context {}

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
        SHA256_FOREIGN_FUNCTION_NAME_CH,
        Sha256HelperOp::Ch as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| ch(args)),
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_MAJ,
        Sha256HelperOp::Maj as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| maj(args)),
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0,
        Sha256HelperOp::LSigma0 as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| lsigma0(args)),
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1,
        Sha256HelperOp::LSigma1 as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| lsigma1(args)),
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0,
        Sha256HelperOp::SSigma0 as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| ssigma0(args)),
        HostPlugin::Sha256,
    )
    .unwrap();

    env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1,
        Sha256HelperOp::SSigma1 as usize,
        Box::new(Context {}),
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        Box::new(|_, args| ssigma1(args)),
        HostPlugin::Sha256,
    )
    .unwrap();
}
