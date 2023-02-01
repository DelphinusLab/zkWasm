use std::rc::Rc;

use specs::{host_function::HostPlugin, types::ValueType};
use wasmi::{RuntimeArgs, RuntimeValue};

use crate::runtime::host::{host_env::HostEnv, ForeignContext};

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
    env.internal_env
        .register_plugin(HostPlugin::Sha256, Box::new(Context {}));

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_CH,
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::Ch as usize,
        Rc::new(|_, args| ch(args)),
    );

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_MAJ,
        specs::host_function::Signature {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::Maj as usize,
        Rc::new(|_, args| maj(args)),
    );

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0,
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::LSigma0 as usize,
        Rc::new(|_, args| lsigma0(args)),
    );

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1,
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::LSigma1 as usize,
        Rc::new(|_, args| lsigma1(args)),
    );

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0,
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::SSigma0 as usize,
        Rc::new(|_, args| ssigma0(args)),
    );

    env.internal_env.register_function(
        SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1,
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(specs::types::ValueType::I32),
        },
        HostPlugin::Sha256,
        Sha256HelperOp::SSigma1 as usize,
        Rc::new(|_, args| ssigma1(args)),
    );
}
