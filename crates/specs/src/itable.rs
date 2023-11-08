use super::mtable::VarType;
use crate::brtable::BrTable;
use crate::brtable::BrTableEntry;
use crate::encode::opcode::encode_br_if_eqz;
use crate::encode::opcode::encode_br_table;
use crate::encode::opcode::encode_call;
use crate::encode::opcode::encode_call_host;
use crate::encode::opcode::encode_call_indirect;
use crate::encode::opcode::encode_conversion;
use crate::encode::opcode::encode_global_get;
use crate::encode::opcode::encode_global_set;
use crate::encode::COMMON_RANGE_OFFSET;
use crate::external_host_call_table::ExternalHostCallSignature;
use crate::host_function::HostPlugin;
use crate::mtable::MemoryReadSize;
use crate::mtable::MemoryStoreSize;
use crate::types::ValueType;
use num_bigint::BigUint;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum OpcodeClass {
    LocalGet = 1,
    LocalSet,
    LocalTee,
    GlobalGet,
    GlobalSet,
    Const,
    Drop,
    Select,
    Return,
    Bin,
    Unary,
    BinShift,
    BinBit,
    Test,
    Rel,
    Br,
    BrIf,
    BrIfEqz,
    BrTable,
    Unreachable,
    Call,
    CallHost,
    CallIndirect,
    Load,
    Store,
    MemorySize,
    MemoryGrow,
    Conversion,
    ForeignPluginStart,
}

impl OpcodeClass {
    pub fn mops(&self) -> u64 {
        match self {
            OpcodeClass::LocalGet => 2,
            OpcodeClass::LocalSet => 2,
            OpcodeClass::LocalTee => 2,
            OpcodeClass::GlobalGet | OpcodeClass::GlobalSet => 2,
            OpcodeClass::Const => 1,
            OpcodeClass::Drop => 0,
            OpcodeClass::Select => 4,
            OpcodeClass::Return => 0,
            OpcodeClass::Bin => 3,
            OpcodeClass::BinShift => 3,
            OpcodeClass::BinBit => 3,
            OpcodeClass::Unary => 2,
            OpcodeClass::Test => 2,
            OpcodeClass::Rel => 3,
            OpcodeClass::Br => 0,
            OpcodeClass::BrIf => 1,
            OpcodeClass::BrIfEqz => 1,
            OpcodeClass::BrTable => 1,
            OpcodeClass::Unreachable => todo!(),
            OpcodeClass::Call => 0,
            OpcodeClass::CallHost => 1, // Push or pop
            OpcodeClass::CallIndirect => 1,
            OpcodeClass::Store => 4, // Load value from stack, load address from stack, read raw value, write value
            OpcodeClass::Load => 3,  // pop address, load memory, push stack
            OpcodeClass::MemorySize => 1,
            OpcodeClass::MemoryGrow => 2,
            OpcodeClass::Conversion => 2,
            OpcodeClass::ForeignPluginStart => 0,
        }
    }

    pub fn jops(&self) -> u64 {
        match self {
            OpcodeClass::Return => 1,
            OpcodeClass::Call => 1,
            OpcodeClass::CallIndirect => 1,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct OpcodeClassPlain(pub usize);

impl OpcodeClassPlain {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnaryOp {
    Ctz,
    Clz,
    Popcnt,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    UnsignedDiv,
    UnsignedRem,
    SignedDiv,
    SignedRem,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShiftOp {
    Shl,
    UnsignedShr,
    SignedShr,
    Rotl,
    Rotr,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, EnumIter, PartialEq, Eq, PartialOrd, Ord)]
pub enum BitOp {
    And = 0,
    Or = 1,
    Xor = 2,
}

impl BitOp {
    pub fn eval(&self, left: u64, right: u64) -> u64 {
        match self {
            BitOp::And => left & right,
            BitOp::Or => left | right,
            BitOp::Xor => left ^ right,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelOp {
    Eq,
    Ne,
    SignedGt,
    UnsignedGt,
    SignedGe,
    UnsignedGe,
    SignedLt,
    UnsignedLt,
    SignedLe,
    UnsignedLe,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, EnumIter, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestOp {
    Eqz,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConversionOp {
    I32WrapI64,
    I64ExtendI32s,
    I64ExtendI32u,
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrTarget {
    pub drop: u32,
    pub keep: Vec<ValueType>,
    pub dst_pc: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Opcode {
    LocalGet {
        vtype: VarType,
        offset: u64,
    },
    LocalSet {
        vtype: VarType,
        offset: u64,
    },
    LocalTee {
        vtype: VarType,
        offset: u64,
    },
    GlobalGet {
        idx: u64,
    },
    GlobalSet {
        idx: u64,
    },
    MemorySize,
    MemoryGrow,
    Const {
        vtype: VarType,
        value: u64,
    },
    Drop,
    Select,
    Return {
        drop: u32,
        keep: Vec<ValueType>,
    },
    Bin {
        class: BinOp,
        vtype: VarType,
    },
    BinShift {
        class: ShiftOp,
        vtype: VarType,
    },
    BinBit {
        class: BitOp,
        vtype: VarType,
    },
    Unary {
        class: UnaryOp,
        vtype: VarType,
    },
    Test {
        class: TestOp,
        vtype: VarType,
    },
    Rel {
        class: RelOp,
        vtype: VarType,
    },
    Br {
        drop: u32,
        keep: Vec<ValueType>,
        dst_pc: u32,
    },
    BrIf {
        drop: u32,
        keep: Vec<ValueType>,
        dst_pc: u32,
    },
    BrIfEqz {
        drop: u32,
        keep: Vec<ValueType>,
        dst_pc: u32,
    },
    BrTable {
        targets: Vec<BrTarget>,
    },
    Unreachable,
    Call {
        index: u32,
    },
    CallIndirect {
        type_idx: u32,
    },
    InternalHostCall {
        plugin: HostPlugin,
        function_index: usize,
        function_name: String,
        op_index_in_plugin: usize,
    },
    ExternalHostCall {
        op: usize,
        sig: ExternalHostCallSignature,
    },
    Load {
        offset: u32,
        vtype: VarType,
        size: MemoryReadSize,
    },
    Store {
        offset: u32,
        vtype: VarType,
        size: MemoryStoreSize,
    },
    Conversion {
        class: ConversionOp,
    },
}

impl Opcode {
    pub fn mops(&self) -> u64 {
        let opcode_class: OpcodeClass = self.clone().into();
        opcode_class.mops()
    }

    pub fn jops(&self) -> u64 {
        let opcode_class: OpcodeClass = self.clone().into();
        opcode_class.jops()
    }

    pub fn vtype(&self) -> Option<VarType> {
        match self {
            Opcode::Const { vtype, .. } => Some(*vtype),
            Opcode::Bin { vtype, .. } => Some(*vtype),
            Opcode::BinBit { vtype, .. } => Some(*vtype),
            _ => None,
        }
    }
}

pub const OPCODE_SHIFT: u32 = OPCODE_CLASS_SHIFT + 16;
pub const OPCODE_CLASS_SHIFT: u32 = OPCODE_ARG0_SHIFT + COMMON_RANGE_OFFSET;
pub const OPCODE_ARG0_SHIFT: u32 = OPCODE_ARG1_SHIFT + COMMON_RANGE_OFFSET;
pub const OPCODE_ARG1_SHIFT: u32 = 64;
pub const OPCODE_CELL: usize = 4;

impl Into<BigUint> for Opcode {
    fn into(self) -> BigUint {
        let bn = match self {
            Opcode::LocalGet { vtype, offset } => {
                (BigUint::from(OpcodeClass::LocalGet as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + offset
            }
            Opcode::LocalSet { vtype, offset } => {
                (BigUint::from(OpcodeClass::LocalSet as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + offset
            }
            Opcode::LocalTee { vtype, offset } => {
                (BigUint::from(OpcodeClass::LocalTee as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + offset
            }
            Opcode::GlobalGet { idx } => encode_global_get(BigUint::from(idx)),
            Opcode::GlobalSet { idx } => encode_global_set(BigUint::from(idx)),
            Opcode::Const { vtype, value } => {
                (BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + value
            }
            Opcode::Drop => BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT,
            Opcode::Select => BigUint::from(OpcodeClass::Select as u64) << OPCODE_CLASS_SHIFT,
            Opcode::Return { drop, keep } => {
                (BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(drop as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(keep.len() as u64) << OPCODE_ARG1_SHIFT)
                    + keep.first().map_or(0u64, |x| VarType::from(*x) as u64)
            }
            Opcode::Bin { class, vtype } => {
                (BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::BinShift { class, vtype } => {
                (BigUint::from(OpcodeClass::BinShift as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::BinBit { class, vtype } => {
                (BigUint::from(OpcodeClass::BinBit as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::Unary { class, vtype } => {
                (BigUint::from(OpcodeClass::Unary as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::Test { class, vtype } => {
                (BigUint::from(OpcodeClass::Test as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::Rel { class, vtype } => {
                (BigUint::from(OpcodeClass::Rel as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
            }
            Opcode::Br { drop, keep, dst_pc } => {
                // TODO: should encode type of keep values?
                (BigUint::from(OpcodeClass::Br as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(drop as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(keep.len() as u64) << OPCODE_ARG1_SHIFT)
                    + dst_pc
            }
            Opcode::BrIf { drop, keep, dst_pc } => {
                // TODO: should encode type of keep values?
                (BigUint::from(OpcodeClass::BrIf as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(drop as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(keep.len() as u64) << OPCODE_ARG1_SHIFT)
                    + dst_pc
            }
            Opcode::BrIfEqz { drop, keep, dst_pc } => encode_br_if_eqz(
                BigUint::from(drop as u64),
                BigUint::from(keep.len() as u64),
                BigUint::from(dst_pc),
            ),
            Opcode::BrTable { targets } => encode_br_table(BigUint::from(targets.len())),
            Opcode::Unreachable => {
                BigUint::from(OpcodeClass::Unreachable as u64) << OPCODE_CLASS_SHIFT
            }
            Opcode::Call { index } => encode_call(BigUint::from(index as u64)),
            Opcode::CallIndirect { type_idx } => {
                encode_call_indirect(BigUint::from(type_idx as u64))
            }
            Opcode::InternalHostCall {
                op_index_in_plugin, ..
            } => {
                let opcode_class_plain: OpcodeClassPlain = self.into();

                (BigUint::from(opcode_class_plain.0) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(op_index_in_plugin as u64))
            }
            Opcode::ExternalHostCall { op, sig } => {
                encode_call_host(BigUint::from(op as u64), BigUint::from(sig.is_ret() as u64))
            }

            Opcode::Load {
                offset,
                vtype,
                size,
            } => {
                (BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(size as u64) << OPCODE_ARG1_SHIFT)
                    + offset
            }
            Opcode::Store {
                offset,
                vtype,
                size,
            } => {
                (BigUint::from(OpcodeClass::Store as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(size as u64) << OPCODE_ARG1_SHIFT)
                    + offset
            }
            Opcode::MemorySize => {
                BigUint::from(OpcodeClass::MemorySize as u64) << OPCODE_CLASS_SHIFT
            }
            Opcode::MemoryGrow => {
                BigUint::from(OpcodeClass::MemoryGrow as u64) << OPCODE_CLASS_SHIFT
            }
            Opcode::Conversion { class } => match class {
                ConversionOp::I32WrapI64 => encode_conversion(
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    1u64.into(),
                    0u64.into(),
                ),
                ConversionOp::I64ExtendI32s => encode_conversion(
                    1u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                ),
                ConversionOp::I64ExtendI32u => encode_conversion(
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                ),
                ConversionOp::I32Extend8S => encode_conversion(
                    1u64.into(),
                    1u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                ),
                ConversionOp::I32Extend16S => encode_conversion(
                    1u64.into(),
                    1u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                ),
                ConversionOp::I64Extend8S => encode_conversion(
                    1u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                ),
                ConversionOp::I64Extend16S => encode_conversion(
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                ),
                ConversionOp::I64Extend32S => encode_conversion(
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                ),
            },
        };
        assert!(bn < BigUint::from(1u64) << OPCODE_SHIFT);
        bn
    }
}

impl Into<OpcodeClass> for Opcode {
    fn into(self) -> OpcodeClass {
        match self {
            Opcode::LocalGet { .. } => OpcodeClass::LocalGet,
            Opcode::LocalSet { .. } => OpcodeClass::LocalSet,
            Opcode::LocalTee { .. } => OpcodeClass::LocalTee,
            Opcode::GlobalGet { .. } => OpcodeClass::GlobalGet,
            Opcode::GlobalSet { .. } => OpcodeClass::GlobalSet,
            Opcode::Const { .. } => OpcodeClass::Const,
            Opcode::Drop { .. } => OpcodeClass::Drop,
            Opcode::Select { .. } => OpcodeClass::Select,
            Opcode::Return { .. } => OpcodeClass::Return,
            Opcode::Bin { .. } => OpcodeClass::Bin,
            Opcode::BinShift { .. } => OpcodeClass::BinShift,
            Opcode::BinBit { .. } => OpcodeClass::BinBit,
            Opcode::Unary { .. } => OpcodeClass::Unary,
            Opcode::Test { .. } => OpcodeClass::Test,
            Opcode::Rel { .. } => OpcodeClass::Rel,
            Opcode::Br { .. } => OpcodeClass::Br,
            Opcode::BrIf { .. } => OpcodeClass::BrIf,
            Opcode::BrIfEqz { .. } => OpcodeClass::BrIfEqz,
            Opcode::BrTable { .. } => OpcodeClass::BrTable,
            Opcode::Unreachable => OpcodeClass::Unreachable,
            Opcode::Call { .. } => OpcodeClass::Call,
            Opcode::CallIndirect { .. } => OpcodeClass::CallIndirect,
            Opcode::InternalHostCall { .. } => OpcodeClass::ForeignPluginStart,
            Opcode::ExternalHostCall { .. } => OpcodeClass::CallHost,
            Opcode::Load { .. } => OpcodeClass::Load,
            Opcode::Store { .. } => OpcodeClass::Store,
            Opcode::MemorySize => OpcodeClass::MemorySize,
            Opcode::MemoryGrow => OpcodeClass::MemoryGrow,
            Opcode::Conversion { .. } => OpcodeClass::Conversion,
        }
    }
}

impl Into<OpcodeClassPlain> for Opcode {
    fn into(self) -> OpcodeClassPlain {
        let class: OpcodeClass = self.clone().into();

        if let Opcode::InternalHostCall { plugin, .. } = self {
            OpcodeClassPlain(class as usize + plugin as usize)
        } else {
            OpcodeClassPlain(class as usize)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionTableEntry {
    pub fid: u32,
    pub iid: u32,
    pub opcode: Opcode,
}

impl InstructionTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn encode_instruction_address(&self) -> BigUint {
        let mut bn = BigUint::from(0u64);
        bn += self.fid;
        bn = bn << 16;
        bn += self.iid;
        bn
    }
}
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct InstructionTable(Vec<InstructionTableEntry>);

impl InstructionTable {
    pub fn new(entries: Vec<InstructionTableEntry>) -> Self {
        InstructionTable(entries)
    }

    pub fn entries(&self) -> &Vec<InstructionTableEntry> {
        &self.0
    }

    pub fn create_brtable(&self) -> BrTable {
        let entries: Vec<Vec<BrTableEntry>> = self
            .entries()
            .iter()
            .map(|entry| match &entry.opcode {
                Opcode::BrTable { targets } => targets
                    .iter()
                    .enumerate()
                    .map(|(index, target)| BrTableEntry {
                        fid: entry.fid,
                        iid: entry.iid,
                        index: index as u32,
                        drop: target.drop,
                        keep: target.keep.len() as u32,
                        dst_pc: target.dst_pc,
                    })
                    .collect(),
                _ => vec![],
            })
            .collect();

        BrTable::new(entries.concat())
    }

    pub fn opcode_class(&self) -> HashSet<OpcodeClassPlain> {
        let mut opcodeclass: HashSet<OpcodeClassPlain> = HashSet::new();

        self.entries().iter().for_each(|entry| {
            opcodeclass.insert(entry.opcode.clone().into());
        });

        opcodeclass
    }

    pub fn push(&mut self, fid: u32, iid: u32, opcode: Opcode) {
        self.0.push(InstructionTableEntry { fid, iid, opcode })
    }
}
