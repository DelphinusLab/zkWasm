use std::{cell::RefCell, rc::Rc};

use wasmi::{ImportsBuilder, ModuleInstance, NopExternals, RuntimeValue};

use crate::{
    runtime::{memory_event_of_step, ExecutionOutcome},
    spec::{etable::EventTableEntry, itable::InstructionTableEntry, CompileTable, ExecutionTable},
};

use super::{
    types::{CompileError, ExecutionError, Value},
    CompileOutcome, WasmRuntime,
};

pub struct WasmiRuntime {}

impl From<Value> for RuntimeValue {
    fn from(v: Value) -> Self {
        match v {
            Value::i32(v) => RuntimeValue::from(v),
            Value::i64(v) => RuntimeValue::from(v),
            Value::u32(v) => RuntimeValue::from(v),
            Value::u64(v) => RuntimeValue::from(v),
        }
    }
}

impl WasmRuntime for WasmiRuntime {
    type Module = wasmi::Module;

    fn new() -> Self {
        WasmiRuntime {}
    }

    fn compile(&self, textual_repr: &str) -> Result<CompileOutcome<Self::Module>, CompileError> {
        let binary = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let module = wasmi::Module::from_buffer(&binary).expect("failed to load wasm");

        let instance = ModuleInstance::new(&module, &ImportsBuilder::default())
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        let mut tracer = wasmi::tracer::Tracer::default();
        tracer.register_module_instance(&instance);

        Ok(CompileOutcome {
            textual_repr: textual_repr.to_string(),
            module,
            tables: CompileTable {
                itable: tracer
                    .itable
                    .0
                    .iter()
                    .map(|ientry| InstructionTableEntry::from(ientry))
                    .collect(),
                imtable: vec![], // TODO
            },
        })
    }

    fn run(
        &self,
        compile_outcome: &CompileOutcome<Self::Module>,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<ExecutionOutcome, ExecutionError> {
        let instance = ModuleInstance::new(&compile_outcome.module, &ImportsBuilder::default())
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        let mut tracer = wasmi::tracer::Tracer::default();
        tracer.register_module_instance(&instance);
        let tracer = Rc::new(RefCell::new(tracer));

        assert_eq!(
            instance
                .invoke_export(
                    function_name,
                    &args
                        .into_iter()
                        .map(|v| RuntimeValue::from(v))
                        .collect::<Vec<_>>(),
                    &mut NopExternals,
                )
                .expect("failed to execute export"),
            None,
        );

        let tracer = tracer.borrow();
        let etable = tracer
            .etable
            .0
            .iter()
            .map(|eentry| EventTableEntry::from(eentry))
            .collect::<Vec<_>>();

        let mtable = etable
            .iter()
            .map(|eentry| memory_event_of_step(eentry))
            .collect::<Vec<Vec<_>>>();
        // concat vectors without Clone
        let mtable = mtable.into_iter().flat_map(|x| x.into_iter()).collect();

        // TODO
        let jtable = vec![];

        Ok(ExecutionOutcome {
            tables: ExecutionTable {
                etable,
                mtable,
                jtable,
            },
        })
    }
}
