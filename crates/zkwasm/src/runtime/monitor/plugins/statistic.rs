use std::cell::RefCell;
use std::rc::Rc;

use parity_wasm::elements::ValueType;
use wasmi::func::FuncInstanceInternal;
use wasmi::isa::Keep;
use wasmi::monitor::Monitor;
use wasmi::runner::InstructionOutcome;
use wasmi::FuncRef;
use wasmi::Trap;
use wasmi::TrapCode;

use super::phantom::PhantomHelper;
use crate::runtime::monitor::Observer;

pub struct StatisticPlugin {
    phantom_helper: PhantomHelper,
    observer: Rc<RefCell<Observer>>,
    instruction_limit: Option<usize>,
}

impl StatisticPlugin {
    pub fn new(
        phantom_regex: &[String],
        wasm_input: FuncRef,
        instruction_limit: Option<usize>,
    ) -> Self {
        Self {
            phantom_helper: PhantomHelper::new(phantom_regex, wasm_input),
            observer: Rc::new(RefCell::new(Observer::default())),
            instruction_limit,
        }
    }

    pub fn expose_observer(&self) -> Rc<RefCell<Observer>> {
        self.observer.clone()
    }
}

impl Monitor for StatisticPlugin {
    fn register_module(
        &mut self,
        module: &parity_wasm::elements::Module,
        module_ref: &wasmi::ModuleRef,
        entry: &str,
    ) -> Result<(), wasmi::Error> {
        self.phantom_helper
            .register_module(module, module_ref, entry)?;

        Ok(())
    }

    fn invoke_instruction_post_hook(
        &mut self,
        fid: u32,
        _iid: u32,
        _sp: u32,
        _allocated_memory_pages: u32,
        value_stack: &wasmi::runner::ValueStack,
        _function_context: &wasmi::runner::FunctionContext,
        _instruction: &wasmi::isa::Instruction,
        outcome: &wasmi::runner::InstructionOutcome,
    ) -> Result<(), Trap> {
        self.observer.borrow_mut().counter +=
            !self.phantom_helper.is_in_phantom_function() as usize;

        if let Some(instruction_limit) = self.instruction_limit {
            if self.observer.borrow_mut().counter > instruction_limit {
                return Err(Trap::Code(TrapCode::InstructionExceedsLimit));
            }
        }

        match outcome {
            InstructionOutcome::ExecuteCall(func_ref) => {
                if let FuncInstanceInternal::Internal { index, .. } = func_ref.as_internal() {
                    if self.phantom_helper.is_phantom_function(*index as u32) {
                        self.observer.borrow_mut().is_in_phantom = true;

                        self.phantom_helper.push_frame(value_stack.len() as u32);
                    }
                }
            }
            InstructionOutcome::Return(drop_keep) => {
                if self.phantom_helper.is_phantom_function(fid) {
                    self.phantom_helper.pop_frame();

                    if !self.phantom_helper.is_in_phantom_function() {
                        self.observer.borrow_mut().is_in_phantom = false;

                        if let Keep::Single(t) = drop_keep.keep {
                            // I32Const
                            self.observer.borrow_mut().counter += 1;
                            // Call wasm_input host function
                            self.observer.borrow_mut().counter += 1;
                            // Convert if needed
                            self.observer.borrow_mut().counter +=
                                (!matches!(t, ValueType::I64)) as usize;
                        }
                        // Return
                        self.observer.borrow_mut().counter += 1;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
