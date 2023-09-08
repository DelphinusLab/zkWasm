use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::circuits::config::zkwasm_k;
use crate::runtime::memory_event_of_step;
use anyhow::Result;
use specs::host_function::HostFunctionDesc;
use specs::jtable::StaticFrameEntry;
use specs::mtable::MTable;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::Tables;
use wasmi::Externals;
use wasmi::ImportResolver;
use wasmi::ModuleInstance;
use wasmi::RuntimeValue;

use super::CompiledImage;
use super::ExecutionResult;

pub struct WasmRuntimeIO {
    pub public_inputs_and_outputs: Rc<RefCell<Vec<u64>>>,
    pub outputs: Rc<RefCell<Vec<u64>>>,
}

impl WasmRuntimeIO {
    pub fn empty() -> Self {
        Self {
            public_inputs_and_outputs: Rc::new(RefCell::new(vec![])),
            outputs: Rc::new(RefCell::new(vec![])),
        }
    }
}

pub trait Execution<R> {
    fn dry_run<E: Externals>(self, externals: &mut E) -> Result<Option<R>>;

    fn run<E: Externals>(
        self,
        externals: &mut E,
        wasm_io: WasmRuntimeIO,
    ) -> Result<ExecutionResult<R>>;
}

impl Execution<RuntimeValue>
    for CompiledImage<wasmi::NotStartedModuleRef<'_>, wasmi::tracer::Tracer>
{
    fn dry_run<E: Externals>(self, externals: &mut E) -> Result<Option<RuntimeValue>> {
        let instance = self.instance.run_start(externals).unwrap();

        let result = instance.invoke_export(&self.entry, &[], externals)?;

        Ok(result)
    }

    fn run<E: Externals>(
        self,
        externals: &mut E,
        wasm_io: WasmRuntimeIO,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        let instance = self
            .instance
            .run_start_tracer(externals, self.tracer.clone())
            .unwrap();

        let result =
            instance.invoke_export_trace(&self.entry, &[], externals, self.tracer.clone())?;

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
            public_inputs_and_outputs: wasm_io.public_inputs_and_outputs.borrow().clone(),
            outputs: wasm_io.outputs.borrow().clone(),
        })
    }
}

pub struct WasmiRuntime;

impl WasmiRuntime {
    pub fn new() -> Self {
        WasmiRuntime
    }

    pub fn compile<'a, I: ImportResolver>(
        module: &'a wasmi::Module,
        imports: &I,
        host_plugin_lookup: &HashMap<usize, HostFunctionDesc>,
        entry: &str,
        phantom_functions: &Vec<String>,
    ) -> Result<CompiledImage<wasmi::NotStartedModuleRef<'a>, wasmi::tracer::Tracer>> {
        let tracer = wasmi::tracer::Tracer::new(host_plugin_lookup.clone(), phantom_functions);
        let tracer = Rc::new(RefCell::new(tracer));

        let instance = ModuleInstance::new(&module, imports, Some(tracer.clone()))
            .expect("failed to instantiate wasm module");

        let fid_of_entry = {
            let idx_of_entry = instance.lookup_function_by_name(tracer.clone(), entry);

            if instance.has_start() {
                tracer
                    .clone()
                    .borrow_mut()
                    .static_jtable_entries
                    .push(StaticFrameEntry {
                        frame_id: 0,
                        next_frame_id: 0,
                        callee_fid: 0, // the fid of start function is always 0
                        fid: idx_of_entry,
                        iid: 0,
                    });
            }

            tracer
                .clone()
                .borrow_mut()
                .static_jtable_entries
                .push(StaticFrameEntry {
                    frame_id: 0,
                    next_frame_id: 0,
                    callee_fid: idx_of_entry,
                    fid: 0,
                    iid: 0,
                });

            if instance.has_start() {
                0
            } else {
                idx_of_entry
            }
        };

        let itable = tracer.borrow().itable.clone();
        let imtable = tracer.borrow().imtable.finalized(zkwasm_k());
        let elem_table = tracer.borrow().elem_table.clone();
        let configure_table = tracer.borrow().configure_table.clone();
        let static_jtable = tracer.borrow().static_jtable_entries.clone();

        Ok(CompiledImage {
            entry: entry.to_owned(),
            tables: CompilationTable {
                itable,
                imtable,
                elem_table,
                configure_table,
                static_jtable,
                fid_of_entry,
            },
            instance,
            tracer,
        })
    }
}
