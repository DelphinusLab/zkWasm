use crate::{
    itable::{BinOp, BitOp, RelOp, ShiftOp},
    mtable::VarType,
    types::ValueType,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum StepInfo {
    BrIfNez {
        condition: i32,
        dst_pc: u32,
        drop: u32,
        keep: Vec<ValueType>,
        drop_values: Vec<u64>,
        keep_values: Vec<u64>,
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
    CallHostTime {
        ret_val: Option<u64>,
    },

    GetLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },
    TeeLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },

    Load {
        vtype: VarType,
        offset: u32,
        raw_address: u32,
        effective_address: u32,
        value: u64,
        block_value: u64,
        mmid: u64,
    },
    Store {
        vtype: VarType,
        offset: u32,
        raw_address: u32,
        effective_address: u32,
        pre_block_value: u64,
        updated_block_value: u64,
        value: u64,
        mmid: u64,
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
    I32BinShiftOp {
        class: ShiftOp,
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
