use crate::types::Value;

#[derive(Debug, Clone)]
pub enum StepInfo {
    BrIfNez {
        value: i32,
        dst_pc: u32,
    },
    Return {
        drop: u32,
        keep: u32,
        drop_values: Vec<u64>,
        keep_values: Vec<u64>,
    },

    Call {
        index: u32,
    },

    GetLocal {
        depth: u32,
        value: u64,
    },

    I32Const {
        value: i32,
    },

    I32BinOp {
        left: i32,
        right: i32,
        value: i32,
    },

    I32Comp {
        left: i32,
        right: i32,
        value: bool,
    },
}
