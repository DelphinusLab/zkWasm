use std::{cell::RefCell, collections::HashMap, rc::Rc};

use specs::{
    etable::EventTable,
    host_function::HostFunctionDesc,
    imtable::InitMemoryTable,
    mtable::MTable,
    types::{CompileError, ExecutionError, Value},
    CompileTable, ExecutionTable,
};
use wasmi::{Externals, ImportResolver, ModuleInstance};

use super::{memory_event_of_step, WasmRuntime};

pub struct WasmiRuntime {
    pub tracer: Rc<RefCell<wasmi::tracer::Tracer>>,
}

impl WasmRuntime for WasmiRuntime {
    type Module = wasmi::Module;
    type Tracer = wasmi::tracer::Tracer;
    type Instance = wasmi::ModuleRef;

    fn new(host_plugin_lookup: HashMap<usize, HostFunctionDesc>) -> Self {
        WasmiRuntime {
            tracer: Rc::new(RefCell::new(wasmi::tracer::Tracer::new(host_plugin_lookup))),
        }
    }

    fn compile_from_wast<I: ImportResolver>(
        &self,
        mut module: wast::core::Module,
        imports: &I,
    ) -> Result<Self::Instance, CompileError> {
        let wasm = module.encode().unwrap();
        self.compile(&wasm, imports)
    }

    fn compile<I: ImportResolver>(
        &self,
        wasm: &Vec<u8>,
        imports: &I,
    ) -> Result<Self::Instance, CompileError> {
        let module = wasmi::Module::from_buffer(wasm).expect("failed to load wasm");

        let instance = ModuleInstance::new(&module, imports, Some(self.tracer.clone()))
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        Ok(instance)
        // let itable = self
        //     .tracer
        //     .borrow()
        //     .itable
        //     .0
        //     .iter()
        //     .map(|ientry| ientry.clone().into())
        //     .collect();
        // let imtable = InitMemoryTable::new(
        //     self.tracer
        //         .borrow()
        //         .imtable
        //         .0
        //         .iter()
        //         .map(|imentry| imentry.clone().into())
        //         .collect(),
        // );

        // Ok(CompileOutcome {
        //     module,
        //     tables: CompileTable { itable, imtable },
        //     instance,
        //     tracer: self.tracer.clone(),
        // })
    }

    fn run<E: Externals>(
        &self,
        externals: &mut E,
        instance: &Self::Instance,
        function_name: &str,
        _public_inputs: Vec<u64>, // TODO: register built-in plugin in current trait
        _private_inputs: Vec<u64>,
    ) -> Result<Option<Value>, ExecutionError> {
        let ret = instance
            .invoke_export_trace(function_name, &[], externals, self.tracer.clone())
            .expect("failed to execute export")
            .map(|v| match v {
                wasmi::RuntimeValue::I32(v) => Value::I32(v),
                wasmi::RuntimeValue::I64(v) => Value::I64(v),
                wasmi::RuntimeValue::F32(_) => unreachable!(),
                wasmi::RuntimeValue::F64(_) => unreachable!(),
            });

        Ok(ret)

        // let tracer = self.tracer.borrow();
        // let etable = tracer
        //     .etable
        //     .get_entries()
        //     .iter()
        //     .map(|eentry| eentry.clone().into())
        //     .collect::<Vec<_>>();

        // let mentries = etable
        //     .iter()
        //     .map(|eentry| memory_event_of_step(eentry, &mut 1))
        //     .collect::<Vec<Vec<_>>>();
        // // concat vectors without Clone
        // let mentries = mentries
        //     .into_iter()
        //     .flat_map(|x| x.into_iter())
        //     .collect::<Vec<_>>();

        // let mut mtable = MTable::new(mentries);
        // mtable.push_accessed_memory_initialization(&compile_outcome.tables.imtable);

        // let jtable = tracer
        //     .jtable
        //     .0
        //     .iter()
        //     .map(|jentry| (*jentry).clone().into())
        //     .collect::<Vec<_>>();

        // Ok(ExecutionOutcome {
        //     tables: ExecutionTable {
        //         etable: EventTable::new(etable),
        //         mtable,
        //         jtable,
        //     },
        // })
    }

    fn compile_table(&self) -> CompileTable {
        let itable = self
            .tracer
            .borrow()
            .itable
            .0
            .iter()
            .map(|ientry| ientry.clone().into())
            .collect();

        let imtable = InitMemoryTable::new(
            self.tracer
                .borrow()
                .imtable
                .0
                .iter()
                .map(|imentry| imentry.clone().into())
                .collect(),
        );

        CompileTable { itable, imtable }
    }

    fn execution_tables(&self) -> ExecutionTable {
        let etable = EventTable::new(
            self.tracer
                .borrow()
                .etable
                .get_entries()
                .iter()
                .map(|eentry| eentry.clone().into())
                .collect::<Vec<_>>(),
        );

        let mentries = etable
            .entries()
            .iter()
            .map(|eentry| memory_event_of_step(eentry, &mut 1))
            .collect::<Vec<Vec<_>>>();
        // concat vectors without Clone
        let mentries = mentries
            .into_iter()
            .flat_map(|x| x.into_iter())
            .collect::<Vec<_>>();

        let mut mtable = MTable::new(mentries);
        mtable.push_accessed_memory_initialization(&self.compile_table().imtable);

        let jtable = self
            .tracer
            .borrow()
            .jtable
            .0
            .iter()
            .map(|jentry| (*jentry).clone().into())
            .collect::<Vec<_>>();

        ExecutionTable {
            etable,
            mtable,
            jtable,
        }
    }
}
