use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::runtime::memory_event_of_step;
use anyhow::Result;
use specs::{
    host_function::HostFunctionDesc, jtable::StaticFrameEntry, mtable::MTable,
    utils::common_range::CommonRange, CompilationTable, ExecutionTable, Tables,
};
use wasmi::{Externals, ImportResolver, ModuleInstance, RuntimeValue};

use super::{CompiledImage, ExecutionResult};

pub trait Execution<R> {
    fn run<E: Externals>(self, externals: &mut E) -> Result<ExecutionResult<R>>;
}

impl Execution<RuntimeValue>
    for CompiledImage<wasmi::NotStartedModuleRef<'_>, wasmi::tracer::Tracer>
{
    fn run<E: Externals>(self, externals: &mut E) -> Result<ExecutionResult<RuntimeValue>> {
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
                    .map(|eentry| memory_event_of_step(eentry, &mut CommonRange::from(1u32)))
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

impl WasmiRuntime {
    pub fn new() -> Self {
        WasmiRuntime
    }

    pub fn compile<'a, I: ImportResolver>(
        &self,
        module: &'a wasmi::Module,
        imports: &I,
        host_plugin_lookup: &HashMap<usize, HostFunctionDesc>,
        entry: &str,
    ) -> Result<CompiledImage<wasmi::NotStartedModuleRef<'a>, wasmi::tracer::Tracer>> {
        let tracer = wasmi::tracer::Tracer::new(host_plugin_lookup.clone());
        let tracer = Rc::new(RefCell::new(tracer));

        let instance = ModuleInstance::new(&module, imports, Some(tracer.clone()))
            .expect("failed to instantiate wasm module");

        {
            let idx_of_entry = instance.lookup_function_by_name(tracer.clone(), entry);

            if instance.has_start() {
                tracer
                    .clone()
                    .borrow_mut()
                    .static_jtable_entries
                    .push(StaticFrameEntry {
                        frame_id: CommonRange::from(0u32),
                        next_frame_id: CommonRange::from(0u32),
                        callee_fid: CommonRange::from(0u32), // the fid of start function is always 0
                        fid: CommonRange::from(idx_of_entry),
                        iid: CommonRange::from(0u32),
                    });
            }

            tracer
                .clone()
                .borrow_mut()
                .static_jtable_entries
                .push(StaticFrameEntry {
                    frame_id: CommonRange::from(0u32),
                    next_frame_id: CommonRange::from(0u32),
                    callee_fid: CommonRange::from(idx_of_entry),
                    fid: CommonRange::from(0u32),
                    iid: CommonRange::from(0u32),
                });
        }

        let itable = tracer.borrow().itable.clone();
        let imtable = tracer.borrow().imtable.finalized();
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
            },
            instance,
            tracer,
        })
    }
}
