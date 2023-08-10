use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use specs::host_function::HostPlugin;
use specs::host_function::Signature;
use specs::types::ValueType;
use wasmi::RuntimeArgs;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;

use super::Op;

fn parse_u64_from_file(path: &PathBuf) -> Vec<u64> {
    let mut inputs = vec![];

    let mut fd = File::open(path.as_path()).unwrap();
    let mut buf = [0u8; 8];

    while let Ok(()) = fd.read_exact(&mut buf) {
        inputs.push(u64::from_le_bytes(buf));
    }

    inputs
}

struct Context {
    inputs: Vec<u64>,
    outputs: Vec<u64>,
    output_file: Option<PathBuf>,
}

impl Context {
    fn new(input_file: Option<PathBuf>, output_file: Option<PathBuf>) -> Self {
        let mut inputs = input_file
            .map(|path| parse_u64_from_file(&path))
            .unwrap_or(vec![]);
        inputs.reverse();

        Context {
            inputs,
            outputs: vec![],
            output_file,
        }
    }

    fn push_output(&mut self, value: u64) {
        self.outputs.push(value)
    }

    fn pop_input(&mut self) -> u64 {
        self.inputs
            .pop()
            .expect("Failed to pop value from context_in array, please check you inputs")
    }
}

impl ForeignContext for Context {
    fn finalized(&self) {
        if let Some(output_file) = &self.output_file {
            let mut fd = std::fs::File::create(output_file).unwrap();

            let mut outputs = self.outputs.clone();
            outputs.reverse();

            while let Some(v) = outputs.pop() {
                fd.write(&v.to_le_bytes()).unwrap();
            }
        }
    }
}

pub fn register_context_foreign(
    env: &mut HostEnv,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
) {
    env.internal_env
        .register_plugin(HostPlugin::Context, Box::new(Context::new(input, output)));

    env.internal_env.register_function(
        "wasm_read_context",
        Signature {
            params: vec![],
            return_type: Some(ValueType::I64),
        },
        HostPlugin::Context,
        Op::ReadContext as usize,
        Rc::new(|context: &mut dyn ForeignContext, _args: RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            Some(wasmi::RuntimeValue::I64(context.pop_input() as i64))
        }),
    );

    env.internal_env.register_function(
        "wasm_write_context",
        Signature {
            params: vec![ValueType::I64],
            return_type: None,
        },
        HostPlugin::Context,
        Op::WriteContext as usize,
        Rc::new(|context: &mut dyn ForeignContext, args: RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            let value: i64 = args.nth(0);
            context.push_output(value as u64);

            None
        }),
    );
}
