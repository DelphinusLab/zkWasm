use crate::{
    itable::{BinOp, BitOp, RelOp},
    mtable::VarType,
    types::ValueType,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum StepInfo {
    BrIfNez {
        value: i32,
        dst_pc: u32,
    },
    Return {
        drop: u32,
        keep: Vec<ValueType>,
        drop_values: Vec<u64>,
        keep_values: Vec<u64>,
    },

    Drop,
    Call {
        index: u16,
    },

    GetLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },

    I32Const {
        value: i32,
    },

    I32BinOp {
        class: BinOp,
        left: i32,
        right: i32,
        value: i32,
    },
    I32BinBitOp {
        class: BitOp,
        left: i32,
        right: i32,
        value: i32,
    },

    I32Comp {
        class: RelOp,
        left: i32,
        right: i32,
        value: bool,
    },
}
