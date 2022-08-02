use super::mtable::VarType;
use crate::types::ValueType;
use num_bigint::BigUint;
use serde::Serialize;
use std::collections::HashSet;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum OpcodeClass {
    LocalGet = 1,
    LocalSet,
    LocalTee,
    Const,
    Drop,
    Return,
    Bin,
    BinShift,
    BinBit,
    Rel,
    Br,
    BrIf,
    Call,
    CallHostTime,
    Load,
    Store,
}

impl OpcodeClass {
    pub fn mops(&self) -> u64 {
        match self {
            OpcodeClass::LocalGet => 2,
            OpcodeClass::LocalSet => 2,
            OpcodeClass::LocalTee => 2,
            OpcodeClass::Const => 1,
            OpcodeClass::Drop => 0,
            OpcodeClass::Return => 0,
            OpcodeClass::Bin => 3,
            OpcodeClass::BinShift => 3,
            OpcodeClass::BinBit => 3,
            OpcodeClass::Rel => 3,
            OpcodeClass::Br => 0,
            OpcodeClass::BrIf => 1,
            OpcodeClass::Call => 0,
            OpcodeClass::CallHostTime => 1,
            OpcodeClass::Store => 4, // Load value from stack, load address from stack, read raw value, write value
            OpcodeClass::Load => 3,  // pop address, load memory, push stack
        }
    }

    pub fn jops(&self) -> u64 {
        match self {
            OpcodeClass::Return => 1,
            OpcodeClass::Call => 1,
            _ => 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum BinOp {
    Add,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum ShiftOp {
    Shl,
    UnsignedShr,
}

#[derive(Copy, Clone, Debug, Serialize, EnumIter)]
pub enum BitOp {
    And = 0,
    Or = 1,
    Xor = 2,
    Not = 3,
}

impl BitOp {
    pub fn eval(&self, left: u64, right: u64) -> u64 {
        match self {
            BitOp::And => left & right,
            BitOp::Or => left | right,
            BitOp::Xor => left ^ right,
            BitOp::Not => !left,
        }
    }

    pub fn is_binop(&self) -> bool {
        match self {
            BitOp::Not => false,
            _ => true,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum RelOp {
    Eq,
    Ne,
    SignedGt,
    UnsignedGt,
    SignedGe,
    UnsignedGe,
}

#[derive(Clone, Debug, Serialize)]
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
    Const {
        vtype: VarType,
        value: u64,
    },
    Drop,
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
    Call {
        index: u16,
    },
    CallHostTime,
    Load {
        offset: u32,
        vtype: VarType,
    },
    Store {
        offset: u32,
        vtype: VarType,
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

pub const OPCODE_CLASS_SHIFT: usize = 96;
pub const OPCODE_ARG0_SHIFT: usize = 80;
pub const OPCODE_ARG1_SHIFT: usize = 64;

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
            Opcode::Const { vtype, value } => {
                (BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + value
            }
            Opcode::Drop => BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT,
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
            Opcode::Call { index } => {
                (BigUint::from(OpcodeClass::Call as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(index as u64) << OPCODE_ARG0_SHIFT)
            }
            Opcode::CallHostTime => {
                BigUint::from(OpcodeClass::CallHostTime as u64) << OPCODE_CLASS_SHIFT
            }
            Opcode::Load { offset, vtype } => {
                (BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + offset
            }
            Opcode::Store { offset, vtype } => {
                (BigUint::from(OpcodeClass::Store as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + offset
            }
        };
        assert!(bn < BigUint::from(1u64) << 128usize);
        bn
    }
}

impl Into<OpcodeClass> for Opcode {
    fn into(self) -> OpcodeClass {
        match self {
            Opcode::LocalGet { .. } => OpcodeClass::LocalGet,
            Opcode::LocalSet { .. } => OpcodeClass::LocalSet,
            Opcode::LocalTee { .. } => OpcodeClass::LocalTee,
            Opcode::Const { .. } => OpcodeClass::Const,
            Opcode::Drop { .. } => OpcodeClass::Drop,
            Opcode::Return { .. } => OpcodeClass::Return,
            Opcode::Bin { .. } => OpcodeClass::Bin,
            Opcode::BinShift { .. } => OpcodeClass::BinShift,
            Opcode::BinBit { .. } => OpcodeClass::BinBit,
            Opcode::Rel { .. } => OpcodeClass::Rel,
            Opcode::Br { .. } => OpcodeClass::Br,
            Opcode::BrIf { .. } => OpcodeClass::BrIf,
            Opcode::Call { .. } => OpcodeClass::Call,
            Opcode::CallHostTime => OpcodeClass::CallHostTime,
            Opcode::Load { .. } => OpcodeClass::Load,
            Opcode::Store { .. } => OpcodeClass::Store,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InstructionTableEntry {
    pub moid: u16,
    pub mmid: u16,
    pub fid: u16,
    pub iid: u16,
    pub opcode: Opcode,
}

impl InstructionTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn encode_instruction_address(&self) -> BigUint {
        let mut bn = BigUint::from(0u64);
        bn += self.moid;
        bn = bn << 16;
        bn += self.fid;
        bn = bn << 16;
        bn += self.iid;
        bn
    }
}

pub fn collect_opcodeclass(ientries: &Vec<InstructionTableEntry>) -> HashSet<OpcodeClass> {
    let mut opcodeclass: HashSet<OpcodeClass> = HashSet::new();
    ientries.iter().for_each(|entry| {
        opcodeclass.insert(entry.opcode.clone().into());
    });
    opcodeclass
}
