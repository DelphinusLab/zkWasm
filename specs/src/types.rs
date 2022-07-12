pub enum ValueType {
    I32,
    I64,
    U32,
    U64,
}
#[derive(Debug, Clone)]
pub enum Value {
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
}

#[derive(Debug)]
pub enum CompileError {}

#[derive(Debug)]
pub enum ExecutionError {}
