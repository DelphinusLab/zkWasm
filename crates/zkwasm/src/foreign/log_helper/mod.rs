use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;
use wasmi::tracer::Observer;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;
use zkwasm_host_circuits::host::ForeignInst::LogChar;

struct Context;
impl ForeignContext for Context {}

pub fn register_log_foreign(env: &mut HostEnv) {
    let foreign_log_plugin = env
        .external_env
        .register_plugin("foreign_print", Box::new(Context));

    let print = Rc::new(
        |_observer: &Observer, _context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            println!("{}", value);
            None
        },
    );

    let printchar = Rc::new(
        |_observer: &Observer, _context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            print!("{}", value as u8 as char);
            None
        },
    );

    env.external_env.register_function(
        "wasm_dbg",
        Log as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin.clone(),
        print,
    );

    env.external_env.register_function(
        "wasm_dbg_char",
        LogChar as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        printchar,
    );
}
