use std::cell::RefCell;
use std::rc::Rc;

use parity_wasm::elements::Module;
use specs::CompilationTable;
use specs::Tables;
use specs::TraceBackend;
use wasmi::isa::Instruction;
use wasmi::monitor::Monitor;
use wasmi::runner::FunctionContext;
use wasmi::runner::InstructionOutcome;
use wasmi::runner::ValueStack;
use wasmi::ModuleImportResolver;
use wasmi::ModuleRef;
use wasmi::RuntimeValue;
use wasmi::Signature;
use wasmi::ValueType;

use crate::runtime::host::host_env::HostEnv;

use super::observer::Observer;
use super::plugins::statistic::StatisticPlugin;
use super::plugins::table::TablePlugin;
use super::WasmiMonitor;

pub struct TableMonitor {
    table_plugin: TablePlugin,
    statistic_plugin: StatisticPlugin,
}

impl TableMonitor {
    pub fn new(k: u32, phantom_regex: &Vec<String>, backend: TraceBackend, env: &HostEnv) -> Self {
        let wasm_input = env
            .resolve_func(
                "wasm_input",
                &Signature::new(vec![ValueType::I32], Some(ValueType::I64)),
            )
            .expect("Failed to resolve wasm_input function, please make sure it is imported in the wasm image.");

        Self {
            table_plugin: TablePlugin::new(
                k,
                env.function_description_table(),
                phantom_regex,
                wasm_input.clone(),
                backend,
            ),
            statistic_plugin: StatisticPlugin::new(phantom_regex, wasm_input),
        }
    }

    pub fn into_compilation_table(self) -> CompilationTable {
        self.table_plugin.into_compilation_table()
    }

    pub fn into_tables(self) -> Tables {
        self.table_plugin.into_tables()
    }
}

impl Monitor for TableMonitor {
    fn register_module(
        &mut self,
        module: &Module,
        module_ref: &ModuleRef,
        entry: &str,
    ) -> Result<(), wasmi::Error> {
        self.table_plugin
            .register_module(module, module_ref, entry)?;
        self.statistic_plugin
            .register_module(module, module_ref, entry)?;

        Ok(())
    }

    fn invoke_exported_function_pre_hook(&mut self) {
        self.table_plugin.invoke_exported_function_pre_hook();
        self.statistic_plugin.invoke_exported_function_pre_hook();
    }

    fn invoke_instruction_pre_hook(
        &mut self,
        value_stack: &ValueStack,
        function_context: &FunctionContext,
        instruction: &Instruction,
    ) {
        self.table_plugin
            .invoke_instruction_pre_hook(value_stack, function_context, instruction);
        self.statistic_plugin.invoke_instruction_pre_hook(
            value_stack,
            function_context,
            instruction,
        );
    }

    fn invoke_instruction_post_hook(
        &mut self,
        fid: u32,
        iid: u32,
        sp: u32,
        allocated_memory_pages: u32,
        value_stack: &ValueStack,
        function_context: &FunctionContext,
        instruction: &Instruction,
        outcome: &InstructionOutcome,
    ) {
        self.table_plugin.invoke_instruction_post_hook(
            fid,
            iid,
            sp,
            allocated_memory_pages,
            value_stack,
            function_context,
            instruction,
            outcome,
        );
        self.statistic_plugin.invoke_instruction_post_hook(
            fid,
            iid,
            sp,
            allocated_memory_pages,
            value_stack,
            function_context,
            instruction,
            outcome,
        );
    }

    fn invoke_call_host_post_hook(&mut self, return_value: Option<RuntimeValue>) {
        self.table_plugin.invoke_call_host_post_hook(return_value);
        self.statistic_plugin
            .invoke_call_host_post_hook(return_value);
    }
}

impl WasmiMonitor for TableMonitor {
    fn expose_observer(&self) -> Rc<RefCell<Observer>> {
        self.statistic_plugin.expose_observer()
    }
}
