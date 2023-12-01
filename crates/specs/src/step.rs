use crate::external_host_call_table::ExternalHostCallSignature;
use crate::host_function::HostPlugin;
use crate::host_function::Signature;
use crate::itable::BinOp;
use crate::itable::BitOp;
use crate::itable::RelOp;
use crate::itable::ShiftOp;
use crate::itable::UnaryOp;
use crate::mtable::MemoryReadSize;
use crate::mtable::MemoryStoreSize;
use crate::mtable::VarType;
use crate::types::ValueType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepInfo {
    Br {
        dst_pc: u32,
        drop: u32,
        keep: Vec<ValueType>,
        keep_values: Vec<u64>,
    },
    BrIfEqz {
        condition: i32,
        dst_pc: u32,
        drop: u32,
        keep: Vec<ValueType>,
        keep_values: Vec<u64>,
    },
    BrIfNez {
        condition: i32,
        dst_pc: u32,
        drop: u32,
        keep: Vec<ValueType>,
        keep_values: Vec<u64>,
    },
    BrTable {
        index: i32,
        dst_pc: u32,
        drop: u32,
        keep: Vec<ValueType>,
        keep_values: Vec<u64>,
    },
    Return {
        drop: u32,
        keep: Vec<ValueType>,
        keep_values: Vec<u64>,
    },

    Drop,
    Select {
        val1: u64,
        val2: u64,
        cond: u64,
        result: u64,
        vtype: VarType,
    },

    Call {
        index: u32,
    },
    CallIndirect {
        table_index: u32,
        type_index: u32,
        offset: u32,
        func_index: u32,
    },
    CallHost {
        plugin: HostPlugin,
        host_function_idx: usize,
        function_name: String,
        signature: Signature,
        args: Vec<u64>,
        ret_val: Option<u64>,
        op_index_in_plugin: usize,
    },
    ExternalHostCall {
        op: usize,
        value: Option<u64>,
        sig: ExternalHostCallSignature,
    },

    GetLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },
    SetLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },
    TeeLocal {
        vtype: VarType,
        depth: u32,
        value: u64,
    },

    GetGlobal {
        idx: u32,
        vtype: VarType,
        is_mutable: bool,
        value: u64,
    },
    SetGlobal {
        idx: u32,
        vtype: VarType,
        is_mutable: bool,
        value: u64,
    },

    Load {
        vtype: VarType,
        load_size: MemoryReadSize,
        offset: u32,
        raw_address: u32,
        effective_address: u32,
        value: u64,
        block_value1: u64,
        block_value2: u64,
    },
    Store {
        vtype: VarType,
        store_size: MemoryStoreSize,
        offset: u32,
        raw_address: u32,
        effective_address: u32,
        pre_block_value1: u64,
        updated_block_value1: u64,
        pre_block_value2: u64,
        updated_block_value2: u64,
        value: u64,
    },

    MemorySize,
    MemoryGrow {
        grow_size: i32,
        result: i32,
    },

    I32Const {
        value: i32,
    },
    I64Const {
        value: i64,
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

    I64BinOp {
        class: BinOp,
        left: i64,
        right: i64,
        value: i64,
    },
    I64BinShiftOp {
        class: ShiftOp,
        left: i64,
        right: i64,
        value: i64,
    },
    I64BinBitOp {
        class: BitOp,
        left: i64,
        right: i64,
        value: i64,
    },

    UnaryOp {
        class: UnaryOp,
        vtype: VarType,
        operand: u64,
        result: u64,
    },

    Test {
        vtype: VarType,
        value: u64,
        result: i32,
    },
    I32Comp {
        class: RelOp,
        left: i32,
        right: i32,
        value: bool,
    },
    I64Comp {
        class: RelOp,
        left: i64,
        right: i64,
        value: bool,
    },

    I32WrapI64 {
        value: i64,
        result: i32,
    },
    I64ExtendI32 {
        value: i32,
        result: i64,
        sign: bool,
    },
    I32SignExtendI8 {
        value: i32,
        result: i32,
    },
    I32SignExtendI16 {
        value: i32,
        result: i32,
    },
    I64SignExtendI8 {
        value: i64,
        result: i64,
    },
    I64SignExtendI16 {
        value: i64,
        result: i64,
    },
    I64SignExtendI32 {
        value: i64,
        result: i64,
    },
}
