use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::runtime::memory_event_of_step;
use anyhow::Result;
use specs::{
    host_function::HostFunctionDesc, mtable::MTable, CompilationTable, ExecutionTable, Tables,
};
use wasmi::{Externals, ImportResolver, ModuleInstance, RuntimeValue};

use super::{CompiledImage, ExecutionResult, WasmRuntime};

pub trait Execution<R> {
    fn run<E: Externals>(
        &self,
        externals: &mut E,
        function_name: &str,
    ) -> Result<ExecutionResult<R>>;
}

impl Execution<RuntimeValue>
    for CompiledImage<wasmi::Module, wasmi::ModuleRef, wasmi::tracer::Tracer>
{
    fn run<E: Externals>(
        &self,
        externals: &mut E,
        function_name: &str,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        let result = self.instance.invoke_export_trace(
            function_name,
            &[],
            externals,
            self.tracer.clone(),
        )?;

        let execution_tables = {
            let tracer = self.tracer.borrow();

            let mtable = {
                let mentries = tracer
                    .etable
                    .entries()
                    .iter()
                    .map(|eentry| memory_event_of_step(eentry, &mut 1))
                    .collect::<Vec<Vec<_>>>()
                    .concat();

                MTable::new(mentries, &self.tables.imtable)
            };

            ExecutionTable {
                etable: tracer.etable.clone(),
                mtable,
                jtable: tracer.jtable.clone(),
            }
        };

        Ok(ExecutionResult {
            tables: Tables {
                compilation_tables: self.tables.clone(),
                execution_tables,
            },
            result,
        })
    }
}

pub struct WasmiRuntime;

impl WasmRuntime for WasmiRuntime {
    type Module = wasmi::Module;
    type Tracer = wasmi::tracer::Tracer;
    type Instance = wasmi::ModuleRef;

    fn new() -> Self {
        WasmiRuntime
    }

    fn compile<I: ImportResolver>(
        &self,
        wasm: &Vec<u8>,
        imports: &I,
        host_plugin_lookup: &HashMap<usize, HostFunctionDesc>,
    ) -> Result<CompiledImage<Self::Module, Self::Instance, Self::Tracer>> {
        let module = wasmi::Module::from_buffer(wasm).expect("failed to load wasm");
        let tracer = wasmi::tracer::Tracer::new(host_plugin_lookup.clone());
        let tracer = Rc::new(RefCell::new(tracer));

        let instance = ModuleInstance::new(&module, imports, Some(tracer.clone()))
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        let itable = tracer.borrow().itable.clone();
        let imtable = tracer.borrow().imtable.finalized();
        let elem_table = tracer.borrow().elem_table.clone();
        let configure_table = tracer.borrow().configure_table.clone();

        Ok(CompiledImage {
            module,
            tables: CompilationTable {
                itable,
                imtable,
                elem_table,
                configure_table,
            },
            instance,
            tracer,
        })
    }
}
