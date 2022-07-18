use super::mtable::VarType;
use crate::types::ValueType;
use num_bigint::BigUint;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpcodeClass {
    LocalGet = 1,
    Const,
    Drop,
    Return,
    Bin,
}

impl OpcodeClass {
    pub fn mops(&self) -> u64 {
        match self {
            OpcodeClass::LocalGet => 1,
            OpcodeClass::Const => 1,
            OpcodeClass::Drop => 0,
            OpcodeClass::Return => 0,
            OpcodeClass::Bin => 3,
        }
    }

    pub fn jops(&self) -> u64 {
        match self {
            OpcodeClass::LocalGet => 0,
            OpcodeClass::Const => 0,
            OpcodeClass::Drop => 0,
            OpcodeClass::Return => 1,
            OpcodeClass::Bin => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Add,
}

#[derive(Clone, Debug)]
pub enum Opcode {
    LocalGet { vtype: VarType, offset: u64 },
    Const { vtype: VarType, value: u64 },
    Drop,
    Return { drop: u32, keep: Vec<ValueType> },
    Bin { class: BinOp, vtype: VarType },
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
            Opcode::Const { vtype, value } => {
                (BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG0_SHIFT)
                    + value
            }
            Opcode::Drop => BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT,
            Opcode::Return { drop, keep } => {
                (BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(drop as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(keep.len() as u64) << OPCODE_ARG0_SHIFT)
                    + keep.first().map_or(0u64, |x| *x as u64)
            }
            Opcode::Bin { class, vtype } => {
                (BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
                    + (BigUint::from(class as u64) << OPCODE_ARG0_SHIFT)
                    + (BigUint::from(vtype as u64) << OPCODE_ARG1_SHIFT)
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
            Opcode::Const { .. } => OpcodeClass::Const,
            Opcode::Drop { .. } => OpcodeClass::Drop,
            Opcode::Return { .. } => OpcodeClass::Return,
            Opcode::Bin { .. } => OpcodeClass::Bin,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstructionTableEntry {
    pub moid: u16,
    pub mmid: u16,
    pub fid: u16,
    pub iid: u16,
    pub opcode: Opcode,
}

impl InstructionTableEntry {
    pub fn encode_instruction_address(&self) -> BigUint {
        let mut bn = BigUint::from(0u64);
        bn += self.moid;
        bn = bn << 16;
        bn += self.fid;
        bn = bn << 16;
        bn += self.iid;
        bn = bn << 16;
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
