use std::cell::RefCell;
use std::rc::Rc;

use parity_wasm::elements::Module;
use wasmi::isa::Instruction;
use wasmi::monitor::Monitor;
use wasmi::runner::FunctionContext;
use wasmi::runner::InstructionOutcome;
use wasmi::runner::ValueStack;
use wasmi::ModuleImportResolver;
use wasmi::ModuleRef;
use wasmi::RuntimeValue;
use wasmi::Signature;
use wasmi::Trap;
use wasmi::ValueType;

use crate::runtime::host::host_env::HostEnv;

use super::observer::Observer;
use super::plugins::statistic::StatisticPlugin;
use super::WasmiMonitor;

pub struct StatisticMonitor {
    statistic_plugin: StatisticPlugin,
}

impl StatisticMonitor {
    pub fn new(phantom_regex: &[String], env: &HostEnv, instruction_limit: Option<usize>) -> Self {
        let wasm_input = env
            .resolve_func(
                "wasm_input",
                &Signature::new(vec![ValueType::I32], Some(ValueType::I64)),
            )
            .expect("Failed to resolve wasm_input function, please make sure it is imported in the wasm image.");

        Self {
            statistic_plugin: StatisticPlugin::new(phantom_regex, wasm_input, instruction_limit),
        }
    }
}

impl Monitor for StatisticMonitor {
    fn register_module(
        &mut self,
        module: &Module,
        module_ref: &ModuleRef,
        entry: &str,
    ) -> Result<(), wasmi::Error> {
        self.statistic_plugin
            .register_module(module, module_ref, entry)?;

        Ok(())
    }

    fn invoke_exported_function_pre_hook(&mut self) {
        self.statistic_plugin.invoke_exported_function_pre_hook();
    }

    fn invoke_instruction_pre_hook(
        &mut self,
        value_stack: &ValueStack,
        function_context: &FunctionContext,
        instruction: &Instruction,
    ) {
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
    ) -> Result<(), Trap> {
        self.statistic_plugin.invoke_instruction_post_hook(
            fid,
            iid,
            sp,
            allocated_memory_pages,
            value_stack,
            function_context,
            instruction,
            outcome,
        )?;

        Ok(())
    }

    fn invoke_call_host_post_hook(&mut self, return_value: Option<RuntimeValue>) {
        self.statistic_plugin
            .invoke_call_host_post_hook(return_value);
    }
}

impl WasmiMonitor for StatisticMonitor {
    fn expose_observer(&self) -> Rc<RefCell<Observer>> {
        self.statistic_plugin.expose_observer()
    }
}
