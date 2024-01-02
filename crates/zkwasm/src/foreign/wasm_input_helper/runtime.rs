use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use specs::host_function::HostPlugin;
use specs::types::ValueType;
use wasmi::tracer::Observer;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::ForeignStatics;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;

use super::Op;

pub struct Context {
    pub public_inputs: Vec<u64>,
    pub private_inputs: VecDeque<u64>,
    pub instance: Rc<RefCell<Vec<u64>>>,
    pub output: Rc<RefCell<Vec<u64>>>,
}

impl Context {
    pub fn new(
        public_inputs: Vec<u64>,
        private_inputs: Vec<u64>,
        instance: Rc<RefCell<Vec<u64>>>,
        output: Rc<RefCell<Vec<u64>>>,
    ) -> Self {
        Context {
            public_inputs,
            private_inputs: private_inputs.into(),
            instance,
            output,
        }
    }

    pub fn pop_public(&mut self) -> u64 {
        if self.public_inputs.is_empty() {
            panic!("failed to read public input, please checkout your input");
        }
        self.public_inputs.remove(0)
    }

    pub fn pop_private(&mut self) -> u64 {
        if self.private_inputs.is_empty() {
            panic!("failed to read private input, please checkout your input");
        }
        self.private_inputs.pop_front().unwrap()
    }

    fn push_public(&mut self, value: u64) {
        let mut instance = self.instance.borrow_mut();
        instance.push(value)
    }

    fn push_output(&mut self, value: u64) {
        let mut instance = self.instance.borrow_mut();
        instance.push(value);

        let mut output = self.output.borrow_mut();
        output.push(value);
    }

    pub fn wasm_input(&mut self, arg: i32) -> u64 {
        assert!(arg == 0 || arg == 1);

        let input = if arg == 1 {
            let value = self.pop_public();
            self.push_public(value);
            value
        } else {
            self.pop_private()
        };

        input
    }

    pub fn wasm_output(&mut self, value: u64) {
        self.push_output(value);
    }
}

impl ForeignContext for Context {
    fn get_statics(&self) -> Option<ForeignStatics> {
        None
    }
}

pub fn register_wasm_input_foreign(
    env: &mut HostEnv,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) -> WasmRuntimeIO {
    let public_inputs_and_outputs = Rc::new(RefCell::new(vec![]));
    let outputs = Rc::new(RefCell::new(vec![]));

    let wasm_input = Rc::new(
        |_observer: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();
            let arg: i32 = args.nth(0);
            let input = context.wasm_input(arg);

            Some(wasmi::RuntimeValue::I64(input as i64))
        },
    );

    let wasm_output = Rc::new(
        |_observer: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            let value: i64 = args.nth(0);
            context.wasm_output(value as u64);

            None
        },
    );

    env.internal_env.register_plugin(
        "wasm input plugin",
        HostPlugin::HostInput,
        Box::new(Context::new(
            public_inputs,
            private_inputs,
            public_inputs_and_outputs.clone(),
            outputs.clone(),
        )),
    );

    env.internal_env.register_function(
        "wasm_input",
        specs::host_function::Signature {
            params: vec![ValueType::I32],
            return_type: Some(ValueType::I64),
        },
        HostPlugin::HostInput,
        Op::WasmInput as usize,
        wasm_input,
    );

    env.internal_env.register_function(
        "wasm_output",
        specs::host_function::Signature {
            params: vec![ValueType::I64],
            return_type: None,
        },
        HostPlugin::HostInput,
        Op::WasmOutput as usize,
        wasm_output,
    );

    WasmRuntimeIO {
        public_inputs_and_outputs,
        outputs,
    }
}
