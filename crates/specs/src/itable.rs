use super::mtable::VarType;
use crate::brtable::BrTable;
use crate::brtable::BrTableEntry;
use crate::encode::opcode::encode_bin;
use crate::encode::opcode::encode_bin_bit;
use crate::encode::opcode::encode_bin_shift;
use crate::encode::opcode::encode_br;
use crate::encode::opcode::encode_br_if;
use crate::encode::opcode::encode_br_if_eqz;
use crate::encode::opcode::encode_br_table;
use crate::encode::opcode::encode_call;
use crate::encode::opcode::encode_call_host;
use crate::encode::opcode::encode_call_indirect;
use crate::encode::opcode::encode_call_internal_host;
use crate::encode::opcode::encode_const;
use crate::encode::opcode::encode_conversion;
use crate::encode::opcode::encode_drop;
use crate::encode::opcode::encode_global_get;
use crate::encode::opcode::encode_global_set;
use crate::encode::opcode::encode_load;
use crate::encode::opcode::encode_local_get;
use crate::encode::opcode::encode_local_set;
use crate::encode::opcode::encode_local_tee;
use crate::encode::opcode::encode_memory_grow;
use crate::encode::opcode::encode_memory_size;
use crate::encode::opcode::encode_rel;
use crate::encode::opcode::encode_return;
use crate::encode::opcode::encode_select;
use crate::encode::opcode::encode_store;
use crate::encode::opcode::encode_test;
use crate::encode::opcode::encode_unary;
use crate::encode::opcode::encode_unreachable;
use crate::external_host_call_table::ExternalHostCallSignature;
use crate::host_function::HostPlugin;
use crate::mtable::MemoryReadSize;
use crate::mtable::MemoryStoreSize;
use crate::types::Value;
use crate::types::ValueType;
use num_bigint::BigUint;
use num_traits::One;
use num_traits::Zero;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UniArg {
    Pop,
    Stack(usize),
    IConst(Value),
}

impl UniArg {
    pub fn is_pop(&self) -> bool {
        matches!(self, UniArg::Pop)
    }

    pub fn get_const_value(&self) -> u64 {
        match self {
            UniArg::Pop => 0,
            UniArg::Stack(_) => 0,
            UniArg::IConst(v) => match v {
                Value::I32(v) => *v as u32 as u64,
                Value::I64(v) => *v as u64,
            },
        }
    }

    pub fn try_decease_stack_depth(&mut self, diff: usize) {
        match self {
            UniArg::Stack(i) => *self = UniArg::Stack(*i - diff),
            _ => {}
        }
    }

    pub fn pop_tag() -> BigUint {
        BigUint::from(0u64) << 64
    }

    pub fn stack_tag() -> BigUint {
        BigUint::from(1u64) << 64
    }

    pub fn i32_const_tag() -> BigUint {
        BigUint::from(2u64) << 64
    }

    pub fn i64_const_tag() -> BigUint {
        BigUint::from(3u64) << 64
    }

    pub(crate) fn encode(&self) -> BigUint {
        macro_rules! tag {
            ($tag:expr, $value:expr) => {
                $tag + $value
            };
        }
        match self {
            UniArg::Pop => tag!(Self::pop_tag(), BigUint::zero()),
            UniArg::Stack(usize) => tag!(Self::stack_tag(), BigUint::from(*usize as u64)),
            UniArg::IConst(c) => match c {
                Value::I32(value) => {
                    tag!(Self::i32_const_tag(), BigUint::from(*value as u32 as u64))
                }
                Value::I64(value) => tag!(Self::i64_const_tag(), BigUint::from(*value as u64)),
            },
        }
    }
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
        uniarg: UniArg,
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
        uniarg: UniArg,
    },
    MemorySize,
    MemoryGrow {
        uniarg: UniArg,
    },
    Const {
        vtype: VarType,
        value: u64,
    },
    Drop,
    Select {
        uniargs: [UniArg; 3],
    },
    Return {
        drop: u32,
        keep: Vec<ValueType>,
    },
    Bin {
        class: BinOp,
        vtype: VarType,
        uniargs: [UniArg; 2],
    },
    BinShift {
        class: ShiftOp,
        vtype: VarType,
        uniargs: [UniArg; 2],
    },
    BinBit {
        class: BitOp,
        vtype: VarType,
        uniargs: [UniArg; 2],
    },
    Unary {
        class: UnaryOp,
        vtype: VarType,
        uniarg: UniArg,
    },
    Test {
        class: TestOp,
        vtype: VarType,
        uniarg: UniArg,
    },
    Rel {
        class: RelOp,
        vtype: VarType,
        uniargs: [UniArg; 2],
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
        uniarg: UniArg,
    },
    BrIfEqz {
        drop: u32,
        keep: Vec<ValueType>,
        dst_pc: u32,
        uniarg: UniArg,
    },
    BrTable {
        targets: Vec<BrTarget>,
        uniarg: UniArg,
    },
    Unreachable,
    Call {
        index: u32,
    },
    CallIndirect {
        type_idx: u32,
        uniarg: UniArg,
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
        uniarg: UniArg,
    },
    Store {
        offset: u32,
        vtype: VarType,
        size: MemoryStoreSize,
        uniargs: [UniArg; 2],
    },
    Conversion {
        class: ConversionOp,
        uniarg: UniArg,
    },
}

impl Opcode {
    pub fn mops(&self) -> u64 {
        let opcode_class: OpcodeClass = self.into();
        opcode_class.mops()
    }

    pub fn jops(&self) -> u64 {
        let opcode_class: OpcodeClass = self.into();
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

pub const OPCODE_SHIFT: u32 = OPCODE_CLASS_SHIFT + 8;
pub const OPCODE_CLASS_SHIFT: u32 = 210;

lazy_static! {
    static ref ENCODE_BOUNDARY: BigUint = BigUint::one() << 250;
}

impl From<&Opcode> for BigUint {
    fn from(opcode: &Opcode) -> BigUint {
        let bn = match opcode {
            Opcode::LocalGet { vtype, offset } => {
                encode_local_get(BigUint::from(*vtype as u64), BigUint::from(*offset))
            }
            Opcode::LocalSet {
                vtype,
                offset,
                uniarg,
            } => encode_local_set(
                BigUint::from(*vtype as u64),
                BigUint::from(*offset),
                uniarg.into(),
            ),
            Opcode::LocalTee { vtype, offset } => {
                encode_local_tee(BigUint::from(*vtype as u64), BigUint::from(*offset))
            }

            Opcode::GlobalGet { idx } => encode_global_get(BigUint::from(*idx)),
            Opcode::GlobalSet { idx, uniarg } => {
                encode_global_set(BigUint::from(*idx), uniarg.into())
            }
            Opcode::Const { vtype, value } => {
                encode_const(BigUint::from(*vtype as u64), BigUint::from(*value))
            }
            Opcode::Drop => encode_drop(),
            Opcode::Select { uniargs } => encode_select(uniargs.into()),
            Opcode::Return { drop, keep } => encode_return(
                BigUint::from(*drop as u64),
                BigUint::from(keep.len() as u64),
                BigUint::from(keep.first().map_or(0u64, |x| VarType::from(x) as u64)),
            ),
            Opcode::Bin {
                class,
                vtype,
                uniargs,
            } => encode_bin(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniargs.into(),
            ),
            Opcode::BinShift {
                class,
                vtype,
                uniargs,
            } => encode_bin_shift(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniargs.into(),
            ),
            Opcode::BinBit {
                class,
                vtype,
                uniargs,
            } => encode_bin_bit(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniargs.into(),
            ),
            Opcode::Unary {
                class,
                vtype,
                uniarg,
            } => encode_unary(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniarg.into(),
            ),
            Opcode::Test {
                class,
                vtype,
                uniarg,
            } => encode_test(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniarg.into(),
            ),
            Opcode::Rel {
                class,
                vtype,
                uniargs,
            } => encode_rel(
                BigUint::from(*class as u64),
                BigUint::from(*vtype as u64),
                uniargs.into(),
            ),
            Opcode::Br { drop, keep, dst_pc } => encode_br(
                BigUint::from(*drop as u64),
                BigUint::from(keep.len() as u64),
                BigUint::from(*dst_pc),
            ),
            Opcode::BrIf {
                drop,
                keep,
                dst_pc,
                uniarg,
            } => encode_br_if(
                BigUint::from(*drop as u64),
                BigUint::from(keep.len() as u64),
                BigUint::from(*dst_pc),
                uniarg.into(),
            ),
            Opcode::BrIfEqz {
                drop,
                keep,
                dst_pc,
                uniarg,
            } => encode_br_if_eqz(
                BigUint::from(*drop as u64),
                BigUint::from(keep.len() as u64),
                BigUint::from(*dst_pc),
                uniarg.into(),
            ),
            Opcode::BrTable { targets, uniarg } => {
                encode_br_table(BigUint::from(targets.len()), uniarg.into())
            }
            Opcode::Unreachable => encode_unreachable(),
            Opcode::Call { index } => encode_call(BigUint::from(*index as u64)),
            Opcode::CallIndirect { type_idx, uniarg } => {
                encode_call_indirect(BigUint::from(*type_idx as u64), uniarg.into())
            }
            Opcode::InternalHostCall {
                op_index_in_plugin, ..
            } => encode_call_internal_host(opcode, *op_index_in_plugin),
            Opcode::ExternalHostCall { op, sig } => encode_call_host(
                BigUint::from(*op as u64),
                BigUint::from(sig.is_ret() as u64),
            ),

            Opcode::Load {
                offset,
                vtype,
                size,
                uniarg,
            } => encode_load(
                BigUint::from(*vtype as u64),
                BigUint::from(*size as u64),
                BigUint::from(*offset),
                uniarg.into(),
            ),
            Opcode::Store {
                offset,
                vtype,
                size,
                uniargs,
            } => encode_store(
                BigUint::from(*vtype as u64),
                BigUint::from(*size as u64),
                BigUint::from(*offset),
                uniargs.into(),
            ),
            Opcode::MemorySize => encode_memory_size(),
            Opcode::MemoryGrow { uniarg } => encode_memory_grow(uniarg.into()),
            Opcode::Conversion { class, uniarg } => match class {
                ConversionOp::I32WrapI64 => encode_conversion(
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    0u64.into(),
                    1u64.into(),
                    1u64.into(),
                    0u64.into(),
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
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
                    uniarg.into(),
                ),
            },
        };

        assert!(bn < BigUint::from(1u64) << OPCODE_SHIFT);

        bn
    }
}

impl From<&Opcode> for OpcodeClass {
    fn from(val: &Opcode) -> Self {
        match val {
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
            Opcode::MemoryGrow { .. } => OpcodeClass::MemoryGrow,
            Opcode::Conversion { .. } => OpcodeClass::Conversion,
        }
    }
}

impl From<&Opcode> for OpcodeClassPlain {
    fn from(val: &Opcode) -> Self {
        let class: OpcodeClass = val.into();

        if let Opcode::InternalHostCall { plugin, .. } = val {
            OpcodeClassPlain(class as usize + (*plugin) as usize)
        } else {
            OpcodeClassPlain(class as usize)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionTableEntry {
    pub fid: u32,
    // pub function_name: String,
    pub iid: u32,
    pub opcode: Opcode,
    pub encode: BigUint,
}

impl Display for InstructionTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl InstructionTableEntry {
    pub fn new(
        fid: u32,
        // function_name: String,
        iid: u32,
        opcode: Opcode,
    ) -> Self {
        let encode = InstructionTableEntry::encode(fid, iid, &opcode);

        Self {
            fid,
            // function_name,
            iid,
            opcode,
            encode,
        }
    }
}

// br_table instructions make itable entry discontinuous
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InstructionTableInternal(Vec<Vec<Option<InstructionTableEntry>>>);

impl InstructionTableInternal {
    pub fn push(&mut self, fid: u32, iid: u32, opcode: Opcode) {
        let fid = fid as usize;
        let iid = iid as usize;

        if self.0.len() <= fid {
            self.0.resize(fid + 1, vec![]);
        }
        if self.0[fid].len() <= iid {
            self.0[fid].resize(iid + 1, None);
        }

        self.0[fid][iid] = Some(InstructionTableEntry::new(
            fid as u32, // function_name,
            iid as u32, opcode,
        ));
    }
}

// Use Option because iid may be discontinuous
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct InstructionTable(Arc<Vec<Vec<Option<InstructionTableEntry>>>>);

impl InstructionTable {
    pub fn new(entries: InstructionTableInternal) -> Self {
        Self(Arc::new(entries.0))
    }

    pub fn get(&self, fid: u32, iid: u32) -> &InstructionTableEntry {
        self.0[fid as usize][iid as usize].as_ref().unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InstructionTableEntry> {
        self.0.iter().flatten().flatten().filter_map(Some)
    }

    pub fn create_brtable(&self) -> BrTable {
        let entries: Vec<Vec<BrTableEntry>> = self
            .iter()
            .map(|entry| match &entry.opcode {
                Opcode::BrTable { targets, .. } => targets
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

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<InstructionTableInternal> for InstructionTable {
    fn from(val: InstructionTableInternal) -> Self {
        InstructionTable::new(val)
    }
}
