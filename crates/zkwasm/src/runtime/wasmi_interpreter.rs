use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use specs::host_function::HostFunctionDesc;
use specs::itable::InstructionTable;
use specs::jtable::StaticFrameEntry;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use specs::state::InitializationState;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::Tables;
use wasmi::ImportResolver;
use wasmi::ModuleInstance;
use wasmi::RuntimeValue;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use super::host::host_env::ExecEnv;
use super::host::host_env::HostEnv;
use super::state::UpdateCompilationTable;
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
    fn run(
        self,
        externals: HostEnv,
        dryrun: bool,
        wasm_io: WasmRuntimeIO,
    ) -> Result<ExecutionResult<R>>;
}

impl Execution<RuntimeValue>
    for CompiledImage<wasmi::NotStartedModuleRef<'_>, wasmi::tracer::Tracer>
{
    fn run(
        self,
        externals: HostEnv,
        dryrun: bool,
        wasm_io: WasmRuntimeIO,
    ) -> Result<ExecutionResult<RuntimeValue>> {
        let mut exec_env = ExecEnv {
            host_env: externals,
            tracer: self.tracer.clone(),
        };
        let instance = self
            .instance
            .run_start_tracer(&mut exec_env, self.tracer.clone())
            .unwrap();

        let result =
            instance.invoke_export_trace(&self.entry, &[], &mut exec_env, self.tracer.clone())?;

        let host_statics = exec_env.host_env.external_env.get_statics();
        // drop to decrease the reference counter of self.tracer
        drop(exec_env);

        let tracer = Rc::try_unwrap(self.tracer).unwrap().into_inner();

        let execution_tables = if !dryrun {
            ExecutionTable {
                etable: Arc::new(tracer.etable),
                jtable: Arc::new(tracer.jtable.clone()),
            }
        } else {
            ExecutionTable::default()
        };

        let updated_init_memory_table = self
            .tables
            .update_init_memory_table(&execution_tables.etable.entries());

        let post_image_table = if !dryrun {
            CompilationTable {
                itable: self.tables.itable.clone(),
                imtable: Arc::new(updated_init_memory_table),
                br_table: self.tables.br_table.clone(),
                elem_table: self.tables.elem_table.clone(),
                configure_table: self.tables.configure_table.clone(),
                static_jtable: self.tables.static_jtable.clone(),
                initialization_state: self
                    .tables
                    .update_initialization_state(&execution_tables.etable.entries(), None),
            }
        } else {
            self.tables.clone()
        };

        Ok(ExecutionResult {
            tables: Tables {
                compilation_tables: self.tables,
                execution_tables,
                post_image_table,
                is_last_slice: true,
            },
            result,
            host_statics,
            guest_statics: tracer.observer.counter,
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
        dry_run: bool,
        phantom_functions: &Vec<String>,
    ) -> Result<CompiledImage<wasmi::NotStartedModuleRef<'a>, wasmi::tracer::Tracer>> {
        let tracer =
            wasmi::tracer::Tracer::new(host_plugin_lookup.clone(), phantom_functions, dry_run);
        let tracer = Rc::new(RefCell::new(tracer));

        let instance = ModuleInstance::new(&module, imports, Some(tracer.clone()))
            .expect("failed to instantiate wasm module");

        let fid_of_entry = {
            let idx_of_entry = instance.lookup_function_by_name(tracer.clone(), entry);

            tracer
                .clone()
                .borrow_mut()
                .static_jtable_entries
                .push(StaticFrameEntry {
                    enable: true,
                    frame_id: 0,
                    next_frame_id: 0,
                    callee_fid: idx_of_entry,
                    fid: 0,
                    iid: 0,
                });

            tracer.as_ref().borrow_mut().static_jtable_entries.push(
                if let Some(idx_of_start_function) = module.module().start_section() {
                    StaticFrameEntry {
                        enable: true,
                        frame_id: 0,
                        next_frame_id: 0,
                        callee_fid: idx_of_start_function,
                        fid: idx_of_entry,
                        iid: 0,
                    }
                } else {
                    StaticFrameEntry {
                        enable: false,
                        frame_id: 0,
                        next_frame_id: 0,
                        callee_fid: 0,
                        fid: 0,
                        iid: 0,
                    }
                },
            );

            if instance.has_start() {
                module.module().start_section().unwrap()
            } else {
                idx_of_entry
            }
        };

        let itable: InstructionTable = tracer.borrow().itable.clone().into();
        // FIXME: avoid clone
        let imtable = tracer.borrow().imtable.finalized();
        let br_table = Arc::new(itable.create_brtable());
        let elem_table = Arc::new(tracer.borrow().elem_table.clone());
        let configure_table = Arc::new(tracer.borrow().configure_table.clone());
        let static_jtable = Arc::new(
            tracer
                .borrow()
                .static_jtable_entries
                .clone()
                .try_into()
                .expect(&format!(
                    "The number of static frame entries should be {}",
                    STATIC_FRAME_ENTRY_NUMBER
                )),
        );
        let initialization_state = InitializationState {
            eid: 1,
            fid: fid_of_entry,
            iid: 0,
            frame_id: 0,
            sp: DEFAULT_VALUE_STACK_LIMIT as u32 - 1,

            host_public_inputs: 1,
            context_in_index: 1,
            context_out_index: 1,
            external_host_call_call_index: 1,

            initial_memory_pages: configure_table.init_memory_pages,
            maximal_memory_pages: configure_table.maximal_memory_pages,

            #[cfg(feature = "continuation")]
            jops: 0,
        };

        Ok(CompiledImage {
            entry: entry.to_owned(),
            tables: CompilationTable {
                itable: Arc::new(itable),
                imtable: Arc::new(imtable),
                br_table,
                elem_table,
                configure_table,
                static_jtable,
                initialization_state,
            },
            instance,
            tracer,
        })
    }
}
