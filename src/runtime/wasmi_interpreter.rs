use std::{cell::RefCell, collections::HashMap, rc::Rc};

use specs::{
    etable::EventTable,
    host_function::HostFunctionDesc,
    imtable::InitMemoryTable,
    itable::InstructionTable,
    mtable::MTable,
    types::{CompileError, ExecutionError},
    CompileTable, ExecutionTable,
};
use wasmi::{Externals, ImportResolver, ModuleInstance};

use crate::runtime::{memory_event_of_step, ExecutionOutcome};

use super::{CompileOutcome, WasmRuntime};

pub struct WasmiRuntime {}

impl WasmRuntime for WasmiRuntime {
    type Module = wasmi::Module;
    type Tracer = wasmi::tracer::Tracer;
    type Instance = wasmi::ModuleRef;

    fn new() -> Self {
        WasmiRuntime {}
    }

    fn compile_from_wast<I: ImportResolver>(
        &self,
        mut module: wast::core::Module,
        imports: &I,
        host_plugin_lookup: &HashMap<usize, HostFunctionDesc>,
    ) -> Result<CompileOutcome<Self::Module, Self::Instance, Self::Tracer>, CompileError> {
        let wasm = module.encode().unwrap();
        self.compile(&wasm, imports, host_plugin_lookup)
    }

    fn compile<I: ImportResolver>(
        &self,
        wasm: &Vec<u8>,
        imports: &I,
        host_plugin_lookup: &HashMap<usize, HostFunctionDesc>,
    ) -> Result<CompileOutcome<Self::Module, Self::Instance, Self::Tracer>, CompileError> {
        let module = wasmi::Module::from_buffer(wasm).expect("failed to load wasm");
        let tracer = wasmi::tracer::Tracer::new(host_plugin_lookup.clone());
        let tracer = Rc::new(RefCell::new(tracer));

        let instance = ModuleInstance::new(&module, imports, Some(tracer.clone()))
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        let itable = InstructionTable::new(
            tracer
                .borrow()
                .itable
                .0
                .iter()
                .map(|ientry| ientry.clone().into())
                .collect(),
        );
        let imtable = InitMemoryTable::new(
            tracer
                .borrow()
                .imtable
                .0
                .iter()
                .map(|imentry| imentry.clone().into())
                .collect(),
        );
        let elem_table = tracer.borrow().elem_table.clone();
        let configure_table = tracer.borrow().configure_table.clone();

        Ok(CompileOutcome {
            module,
            tables: CompileTable {
                itable,
                imtable,
                elem_table,
                configure_table,
            },
            instance,
            tracer,
        })
    }

    fn run<E: Externals>(
        &self,
        externals: &mut E,
        compile_outcome: &CompileOutcome<Self::Module, Self::Instance, Self::Tracer>,
        function_name: &str,
        _public_inputs: Vec<u64>, // TODO: register built-in plugin in current trait
        _private_inputs: Vec<u64>,
    ) -> Result<ExecutionOutcome, ExecutionError> {
        compile_outcome
            .instance
            .invoke_export_trace(
                function_name,
                &[],
                externals,
                compile_outcome.tracer.clone(),
            )
            .expect("failed to execute export");

        let tracer = compile_outcome.tracer.borrow();
        let etable = tracer
            .etable
            .get_entries()
            .iter()
            .map(|eentry| eentry.clone().into())
            .collect::<Vec<_>>();

        let mentries = etable
            .iter()
            .map(|eentry| memory_event_of_step(eentry, &mut 1))
            .collect::<Vec<Vec<_>>>();
        // concat vectors without Clone
        let mentries = mentries
            .into_iter()
            .flat_map(|x| x.into_iter())
            .collect::<Vec<_>>();

        let mut mtable = MTable::new(mentries);
        mtable.push_accessed_memory_initialization(&compile_outcome.tables.imtable);

        let jtable = tracer
            .jtable
            .0
            .iter()
            .map(|jentry| (*jentry).clone().into())
            .collect::<Vec<_>>();

        Ok(ExecutionOutcome {
            tables: ExecutionTable {
                etable: EventTable::new(etable),
                mtable,
                jtable,
            },
        })
    }
}
