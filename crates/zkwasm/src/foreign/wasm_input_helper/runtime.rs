use std::cell::RefCell;
use std::rc::Rc;

use specs::host_function::HostPlugin;
use specs::types::ValueType;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;

use super::Op;

pub struct Context {
    pub public_inputs: Vec<u64>,
    pub private_inputs: Vec<u64>,
    pub witness_queue: Vec<u64>,
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
            private_inputs,
            witness_queue: vec![],
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
        self.private_inputs.remove(0)
    }

    fn inject_witness(&mut self, v: u64) {
        self.witness_queue.insert(0, v);
    }

    fn pop_witness(&mut self) -> u64 {
        self.witness_queue.pop().unwrap()
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

    pub fn wasm_witness_inject(&mut self, arg: u64) {
        self.inject_witness(arg);
    }

    pub fn wasm_witness_pop(&mut self) -> u64 {
        self.pop_witness()
    }


    pub fn wasm_output(&mut self, value: u64) {
        self.push_output(value);
    }
}

impl ForeignContext for Context {}

pub fn register_wasm_input_foreign(
    env: &mut HostEnv,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) -> WasmRuntimeIO {
    let public_inputs_and_outputs = Rc::new(RefCell::new(vec![]));
    let outputs = Rc::new(RefCell::new(vec![]));

    let wasm_input = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            let arg: i32 = args.nth(0);
            let input = context.wasm_input(arg);

            Some(wasmi::RuntimeValue::I64(input as i64))
        },
    );

    let wasm_witness_inject = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();
            let value: i64 = args.nth(0);
            context.wasm_witness_inject(value as u64);
            None
        },
    );

    let wasm_witness_pop = Rc::new(
        |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();
            let input = context.wasm_witness_pop();
            Some(wasmi::RuntimeValue::I64(input as i64))
        },
    );

    let wasm_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<Context>().unwrap();

            let value: i64 = args.nth(0);
            context.wasm_output(value as u64);

            None
        },
    );

    env.internal_env.register_plugin(
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
        "wasm_witness_inject",
        specs::host_function::Signature {
            params: vec![ValueType::I64],
            return_type: None,
        },
        HostPlugin::HostInput,
        Op::WasmWitnessInject as usize,
        wasm_witness_inject,
    );

    env.internal_env.register_function(
        "wasm_witness_pop",
        specs::host_function::Signature {
            params: vec![],
            return_type: Some(ValueType::I64),
        },
        HostPlugin::HostInput,
        Op::WasmWitnessPop as usize,
        wasm_witness_pop,
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
