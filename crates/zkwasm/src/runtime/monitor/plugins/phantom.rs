use std::collections::HashSet;

use parity_wasm::elements::Module;
use regex::Regex;
use wasmi::monitor::Monitor;
use wasmi::FuncRef;
use wasmi::ModuleRef;

pub struct PhantomHelper {
    phantom_regex: Vec<Regex>,
    phantom_functions: HashSet<u32>,

    pub(in crate::runtime::monitor) wasm_input: FuncRef,
    frame: Vec<u32>,
}

impl PhantomHelper {
    pub fn new(phantom_regex: &Vec<String>, wasm_input: FuncRef) -> Self {
        Self {
            phantom_regex: phantom_regex
                .iter()
                .map(|s| Regex::new(s).unwrap())
                .collect::<Vec<_>>(),
            phantom_functions: HashSet::new(),

            wasm_input,

            frame: Vec::new(),
        }
    }

    pub(in crate::runtime::monitor) fn is_in_phantom_function(&self) -> bool {
        self.frame.len() > 0
    }

    pub(in crate::runtime::monitor) fn is_phantom_function(&self, func_index: u32) -> bool {
        self.phantom_functions.contains(&func_index)
    }

    pub(in crate::runtime::monitor) fn wasm_input_func_idx(&self, module_ref: &ModuleRef) -> u32 {
        module_ref.func_index_by_func_ref(&self.wasm_input)
    }

    pub(in crate::runtime::monitor) fn push_frame(&mut self, sp: u32) {
        self.frame.push(sp)
    }

    pub(in crate::runtime::monitor) fn pop_frame(&mut self) -> Option<u32> {
        self.frame.pop()
    }
}

impl Monitor for PhantomHelper {
    fn register_module(
        &mut self,
        _module: &Module,
        module_ref: &ModuleRef,
        _entry: &str,
    ) -> Result<(), wasmi::Error> {
        module_ref
            .exports
            .borrow()
            .iter()
            .for_each(|(name, export)| {
                if export.as_func().is_some()
                    && self.phantom_regex.iter().any(|re| re.is_match(name))
                {
                    self.phantom_functions
                        .insert(module_ref.func_index_by_func_ref(export.as_func().unwrap()));
                }
            });

        Ok(())
    }
}
