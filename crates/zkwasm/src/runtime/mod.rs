use std::cell::RefCell;
use std::rc::Rc;

use specs::CompilationTable;
use specs::Tables;

use self::wasmi_interpreter::WasmiRuntime;

pub mod host;
// pub mod state;
pub mod wasmi_interpreter;

pub struct CompiledImage<I, T> {
    pub entry: String,
    pub tables: CompilationTable,
    pub instance: I,
    pub tracer: Rc<RefCell<T>>,
}

#[derive(Clone)]
pub struct ExecutionResult<R> {
    pub tables: Option<Tables>,
    pub result: Option<R>,
    pub public_inputs_and_outputs: Vec<u64>,
    pub outputs: Vec<u64>,
}

// TODO: use feature
pub type WasmInterpreter = WasmiRuntime;
