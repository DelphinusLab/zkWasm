pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;

enum Op {
    ReadContext = 0,
    WriteContext = 1,
}
