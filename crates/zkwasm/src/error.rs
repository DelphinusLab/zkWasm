use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilationError {}

#[derive(Debug, Error)]
pub enum ExecutionError {}

#[derive(Debug, Error)]
pub enum BuildingCircuitError {
    #[error("Only support single slice for non-continuation mode but {0} provided. You could increase K or enable continuation feature.")]
    MultiSlicesNotSupport(usize),
    #[error("Allocated pages({0}) exceed the limit({1}). Current K is {2}, consider increasing the circuit size K.")]
    PagesExceedLimit(u32, u32, u32),
    #[error("Etable entries({0}) exceed the limit({1}). Current K is {2}, consider increasing the circuit size K.")]
    EtableEntriesExceedLimit(u32, u32, u32),
}
