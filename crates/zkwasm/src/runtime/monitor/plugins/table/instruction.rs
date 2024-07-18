use parity_wasm::elements::ValueType;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::itable::BinOp;
use specs::itable::BinaryOp;
use specs::itable::BitOp;
use specs::itable::BrTarget;
use specs::itable::ConversionOp;
use specs::itable::Opcode;
use specs::itable::RelOp;
use specs::itable::ShiftOp;
use specs::itable::TestOp;
use specs::itable::UnaryOp;
use specs::itable::UniArg;
use specs::mtable::MemoryReadSize;
use specs::mtable::MemoryStoreSize;
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::types::FunctionType;
use wasmi::isa;
use wasmi::isa::DropKeep;
use wasmi::isa::Instruction;
use wasmi::isa::InstructionInternal;
use wasmi::isa::Keep;
use wasmi::isa::Target;
use wasmi::runner::effective_address;
use wasmi::runner::from_value_internal_to_u64_with_typ;
use wasmi::runner::FromValueInternal;
use wasmi::runner::FunctionContext;
use wasmi::runner::ValueInternal;
use wasmi::runner::ValueStack;
use wasmi::ModuleRef;
use wasmi::Signature;

use super::TablePlugin;
use super::DEFAULT_TABLE_INDEX;

// uniargs: from nearest
pub fn value_from_uniargs(uniargs: &[UniArg], stack: &ValueStack) -> Vec<ValueInternal> {
    uniargs
        .iter()
        .fold(
            (0usize, vec![]),
            |(delta, mut values), uniarg| match uniarg {
                UniArg::Pop => {
                    values.push(*stack.pick(delta + 1));
                    (delta + 1, values)
                }
                UniArg::Stack(depth) => {
                    values.push(*stack.pick(depth + delta));
                    (delta, values)
                }
                UniArg::IConst(value) => {
                    values.push(value.into());
                    (delta, values)
                }
            },
        )
        .1
}

#[derive(Debug)]
pub struct FuncDesc {
    pub ftype: FunctionType,
    pub signature: Signature,
}

pub struct PhantomFunction;

impl PhantomFunction {
    pub fn build_phantom_function_instructions(
        sig: &Signature,
        // Wasm Image Function Id
        wasm_input_function_idx: u32,
    ) -> Vec<Instruction<'static>> {
        let mut instructions = vec![];

        if sig.return_type().is_some() {
            instructions.push(Instruction::I32Const(0));

            instructions.push(Instruction::Call(wasm_input_function_idx));

            if sig.return_type() != Some(wasmi::ValueType::I64) {
                instructions.push(Instruction::I32WrapI64(UniArg::Pop));
            }
        }

        instructions.push(Instruction::Return(DropKeep {
            drop: sig.params().len() as u32,
            keep: if let Some(t) = sig.return_type() {
                Keep::Single(t.into_elements())
            } else {
                Keep::None
            },
        }));

        instructions
    }
}

pub(super) trait InstructionIntoOpcode {
    fn into_opcode<'a>(self, function_mapping: &impl Fn(u32) -> &'a FuncDesc) -> Opcode;
}

impl<'a> InstructionIntoOpcode for wasmi::isa::Instruction<'a> {
    fn into_opcode<'b>(self, function_mapping: &impl Fn(u32) -> &'b FuncDesc) -> Opcode {
        match self {
            Instruction::GetLocal(offset, typ) => Opcode::LocalGet {
                offset: offset as u64,
                vtype: typ.into(),
            },
            Instruction::SetLocal(offset, typ, uniarg) => Opcode::LocalSet {
                offset: offset as u64,
                vtype: typ.into(),
                uniarg,
            },
            Instruction::TeeLocal(offset, typ, ..) => Opcode::LocalTee {
                offset: offset as u64,
                vtype: typ.into(),
            },
            Instruction::Br(Target { dst_pc, drop_keep }) => Opcode::Br {
                drop: drop_keep.drop,
                keep: if let Keep::Single(t) = drop_keep.keep {
                    vec![t.into()]
                } else {
                    vec![]
                },
                dst_pc,
            },
            Instruction::BrIfEqz(Target { dst_pc, drop_keep }, uniarg) => Opcode::BrIfEqz {
                drop: drop_keep.drop,
                keep: if let Keep::Single(t) = drop_keep.keep {
                    vec![t.into()]
                } else {
                    vec![]
                },
                dst_pc,
                uniarg,
            },
            Instruction::BrIfNez(Target { dst_pc, drop_keep }, uniarg) => Opcode::BrIf {
                drop: drop_keep.drop,
                keep: if let Keep::Single(t) = drop_keep.keep {
                    vec![t.into()]
                } else {
                    vec![]
                },
                dst_pc,
                uniarg,
            },
            Instruction::BrTable(targets, uniarg) => Opcode::BrTable {
                targets: targets
                    .stream
                    .iter()
                    .map(|t| {
                        if let InstructionInternal::BrTableTarget(target) = t {
                            let keep_type = match target.drop_keep.keep {
                                Keep::None => vec![],
                                Keep::Single(t) => vec![t.into()],
                            };

                            BrTarget {
                                drop: target.drop_keep.drop,
                                keep: keep_type,
                                dst_pc: target.dst_pc,
                            }
                        } else {
                            unreachable!()
                        }
                    })
                    .collect(),
                uniarg,
            },
            Instruction::Unreachable => Opcode::Unreachable,
            Instruction::Return(drop_keep) => Opcode::Return {
                drop: drop_keep.drop,
                keep: if let Keep::Single(t) = drop_keep.keep {
                    vec![t.into()]
                } else {
                    vec![]
                },
            },
            Instruction::Call(func_index) => {
                let func_desc = function_mapping(func_index);

                match &func_desc.ftype {
                    specs::types::FunctionType::WasmFunction => Opcode::Call { index: func_index },
                    specs::types::FunctionType::HostFunction {
                        plugin,
                        function_index,
                        function_name,
                        op_index_in_plugin,
                    } => Opcode::InternalHostCall {
                        plugin: *plugin,
                        function_index: *function_index,
                        function_name: function_name.clone(),
                        op_index_in_plugin: *op_index_in_plugin,
                    },
                    specs::types::FunctionType::HostFunctionExternal { op, sig, .. } => {
                        Opcode::ExternalHostCall { op: *op, sig: *sig }
                    }
                }
            }
            Instruction::CallIndirect(idx, uniarg) => Opcode::CallIndirect {
                type_idx: idx,
                uniarg,
            },
            Instruction::Drop => Opcode::Drop,
            Instruction::Select(_, lhs, rhs, cond) => Opcode::Select {
                uniargs: [lhs, rhs, cond],
            },
            Instruction::GetGlobal(idx, ..) => Opcode::GlobalGet { idx: idx as u64 },
            Instruction::SetGlobal(idx, uniarg) => Opcode::GlobalSet {
                idx: idx as u64,
                uniarg,
            },
            Instruction::I32Load(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I32,
                size: MemoryReadSize::U32,
                uniarg,
            },
            Instruction::I64Load(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::I64,
                uniarg,
            },
            Instruction::F32Load(_) => todo!(),
            Instruction::F64Load(_) => todo!(),
            Instruction::I32Load8S(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I32,
                size: MemoryReadSize::S8,
                uniarg,
            },
            Instruction::I32Load8U(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I32,
                size: MemoryReadSize::U8,
                uniarg,
            },
            Instruction::I32Load16S(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I32,
                size: MemoryReadSize::S16,
                uniarg,
            },
            Instruction::I32Load16U(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I32,
                size: MemoryReadSize::U16,
                uniarg,
            },
            Instruction::I64Load8S(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::S8,
                uniarg,
            },
            Instruction::I64Load8U(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::U8,
                uniarg,
            },
            Instruction::I64Load16S(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::S16,
                uniarg,
            },
            Instruction::I64Load16U(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::U16,
                uniarg,
            },
            Instruction::I64Load32S(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::S32,
                uniarg,
            },
            Instruction::I64Load32U(offset, uniarg) => Opcode::Load {
                offset,
                vtype: VarType::I64,
                size: MemoryReadSize::U32,
                uniarg,
            },
            Instruction::I32Store(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I32,
                size: MemoryStoreSize::Byte32,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Store(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I64,
                size: MemoryStoreSize::Byte64,
                uniargs: [arg0, arg1],
            },
            Instruction::F32Store(_) => todo!(),
            Instruction::F64Store(_) => todo!(),
            Instruction::I32Store8(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I32,
                size: MemoryStoreSize::Byte8,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Store16(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I32,
                size: MemoryStoreSize::Byte16,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Store8(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I64,
                size: MemoryStoreSize::Byte8,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Store16(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I64,
                size: MemoryStoreSize::Byte16,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Store32(offset, arg0, arg1) => Opcode::Store {
                offset,
                vtype: VarType::I64,
                size: MemoryStoreSize::Byte32,
                uniargs: [arg0, arg1],
            },
            Instruction::CurrentMemory => Opcode::MemorySize,
            Instruction::GrowMemory(uniarg) => Opcode::MemoryGrow { uniarg },
            Instruction::I32Const(v) => Opcode::Const {
                vtype: VarType::I32,
                value: v as u32 as u64,
            },
            Instruction::I64Const(v) => Opcode::Const {
                vtype: VarType::I64,
                value: v as u64,
            },
            Instruction::F32Const(_) => todo!(),
            Instruction::F64Const(_) => todo!(),
            Instruction::I32Eqz(uniarg) => Opcode::Test {
                class: TestOp::Eqz,
                vtype: VarType::I32,
                uniarg,
            },
            Instruction::I32Eq(arg0, arg1) => Opcode::Rel {
                class: RelOp::Eq,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Ne(arg0, arg1) => Opcode::Rel {
                class: RelOp::Ne,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32LtS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedLt,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32LtU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedLt,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32GtS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedGt,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32GtU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedGt,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32LeS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedLe,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32LeU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedLe,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32GeS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedGe,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32GeU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedGe,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Eqz(uniarg) => Opcode::Test {
                class: TestOp::Eqz,
                vtype: VarType::I64,
                uniarg,
            },
            Instruction::I64Eq(arg0, arg1) => Opcode::Rel {
                class: RelOp::Eq,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Ne(arg0, arg1) => Opcode::Rel {
                class: RelOp::Ne,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64LtS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedLt,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64LtU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedLt,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64GtS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedGt,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64GtU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedGt,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64LeS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedLe,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64LeU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedLe,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64GeS(arg0, arg1) => Opcode::Rel {
                class: RelOp::SignedGe,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64GeU(arg0, arg1) => Opcode::Rel {
                class: RelOp::UnsignedGe,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::F32Eq => todo!(),
            Instruction::F32Ne => todo!(),
            Instruction::F32Lt => todo!(),
            Instruction::F32Gt => todo!(),
            Instruction::F32Le => todo!(),
            Instruction::F32Ge => todo!(),
            Instruction::F64Eq => todo!(),
            Instruction::F64Ne => todo!(),
            Instruction::F64Lt => todo!(),
            Instruction::F64Gt => todo!(),
            Instruction::F64Le => todo!(),
            Instruction::F64Ge => todo!(),
            Instruction::I32Clz(uniarg) => Opcode::Unary {
                class: UnaryOp::Clz,
                vtype: VarType::I32,
                uniarg,
            },
            Instruction::I32Ctz(uniarg) => Opcode::Unary {
                class: UnaryOp::Ctz,
                vtype: VarType::I32,
                uniarg,
            },
            Instruction::I32Popcnt(uniarg) => Opcode::Unary {
                class: UnaryOp::Popcnt,
                vtype: VarType::I32,
                uniarg,
            },

            Instruction::I32Add(lhs, rhs)
            | Instruction::I32Sub(lhs, rhs)
            | Instruction::I32Mul(lhs, rhs)
            | Instruction::I32DivS(lhs, rhs)
            | Instruction::I32DivU(lhs, rhs)
            | Instruction::I32RemS(lhs, rhs)
            | Instruction::I32RemU(lhs, rhs) => Opcode::Bin {
                class: BinOp::from(&self),
                vtype: VarType::I32,
                uniargs: [lhs, rhs],
            },

            Instruction::I32And(arg0, arg1) => Opcode::BinBit {
                class: BitOp::And,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Or(arg0, arg1) => Opcode::BinBit {
                class: BitOp::Or,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Xor(arg0, arg1) => Opcode::BinBit {
                class: BitOp::Xor,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Shl(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Shl,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32ShrS(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::SignedShr,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32ShrU(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::UnsignedShr,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Rotl(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Rotl,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I32Rotr(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Rotr,
                vtype: VarType::I32,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Clz(uniarg) => Opcode::Unary {
                class: UnaryOp::Clz,
                vtype: VarType::I64,
                uniarg,
            },
            Instruction::I64Ctz(uniarg) => Opcode::Unary {
                class: UnaryOp::Ctz,
                vtype: VarType::I64,
                uniarg,
            },
            Instruction::I64Popcnt(uniarg) => Opcode::Unary {
                class: UnaryOp::Popcnt,
                vtype: VarType::I64,
                uniarg,
            },
            Instruction::I64Add(arg0, arg1) => Opcode::Bin {
                class: BinOp::Add,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Sub(arg0, arg1) => Opcode::Bin {
                class: BinOp::Sub,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Mul(arg0, arg1) => Opcode::Bin {
                class: BinOp::Mul,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64DivS(arg0, arg1) => Opcode::Bin {
                class: BinOp::SignedDiv,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64DivU(arg0, arg1) => Opcode::Bin {
                class: BinOp::UnsignedDiv,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64RemS(arg0, arg1) => Opcode::Bin {
                class: BinOp::SignedRem,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64RemU(arg0, arg1) => Opcode::Bin {
                class: BinOp::UnsignedRem,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64And(arg0, arg1) => Opcode::BinBit {
                class: BitOp::And,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Or(arg0, arg1) => Opcode::BinBit {
                class: BitOp::Or,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Xor(arg0, arg1) => Opcode::BinBit {
                class: BitOp::Xor,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Shl(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Shl,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64ShrS(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::SignedShr,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64ShrU(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::UnsignedShr,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Rotl(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Rotl,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::I64Rotr(arg0, arg1) => Opcode::BinShift {
                class: ShiftOp::Rotr,
                vtype: VarType::I64,
                uniargs: [arg0, arg1],
            },
            Instruction::F32Abs => todo!(),
            Instruction::F32Neg => todo!(),
            Instruction::F32Ceil => todo!(),
            Instruction::F32Floor => todo!(),
            Instruction::F32Trunc => todo!(),
            Instruction::F32Nearest => todo!(),
            Instruction::F32Sqrt => todo!(),
            Instruction::F32Add => todo!(),
            Instruction::F32Sub => todo!(),
            Instruction::F32Mul => todo!(),
            Instruction::F32Div => todo!(),
            Instruction::F32Min => todo!(),
            Instruction::F32Max => todo!(),
            Instruction::F32Copysign => todo!(),
            Instruction::F64Abs => todo!(),
            Instruction::F64Neg => todo!(),
            Instruction::F64Ceil => todo!(),
            Instruction::F64Floor => todo!(),
            Instruction::F64Trunc => todo!(),
            Instruction::F64Nearest => todo!(),
            Instruction::F64Sqrt => todo!(),
            Instruction::F64Add => todo!(),
            Instruction::F64Sub => todo!(),
            Instruction::F64Mul => todo!(),
            Instruction::F64Div => todo!(),
            Instruction::F64Min => todo!(),
            Instruction::F64Max => todo!(),
            Instruction::F64Copysign => todo!(),
            Instruction::I32WrapI64(uniarg) => Opcode::Conversion {
                class: ConversionOp::I32WrapI64,
                uniarg,
            },
            Instruction::I32TruncSF32 => todo!(),
            Instruction::I32TruncUF32 => todo!(),
            Instruction::I32TruncSF64 => todo!(),
            Instruction::I32TruncUF64 => todo!(),
            Instruction::I64ExtendSI32(uniarg) => Opcode::Conversion {
                class: ConversionOp::I64ExtendI32s,
                uniarg,
            },
            Instruction::I64ExtendUI32(uniarg) => Opcode::Conversion {
                class: ConversionOp::I64ExtendI32u,
                uniarg,
            },
            Instruction::I64TruncSF32 => todo!(),
            Instruction::I64TruncUF32 => todo!(),
            Instruction::I64TruncSF64 => todo!(),
            Instruction::I64TruncUF64 => todo!(),
            Instruction::F32ConvertSI32 => todo!(),
            Instruction::F32ConvertUI32 => todo!(),
            Instruction::F32ConvertSI64 => todo!(),
            Instruction::F32ConvertUI64 => todo!(),
            Instruction::F32DemoteF64 => todo!(),
            Instruction::F64ConvertSI32 => todo!(),
            Instruction::F64ConvertUI32 => todo!(),
            Instruction::F64ConvertSI64 => todo!(),
            Instruction::F64ConvertUI64 => todo!(),
            Instruction::F64PromoteF32 => todo!(),
            Instruction::I32ReinterpretF32 => todo!(),
            Instruction::I64ReinterpretF64 => todo!(),
            Instruction::F32ReinterpretI32 => todo!(),
            Instruction::F64ReinterpretI64 => todo!(),
            Instruction::I32Extend8S(uniarg) => Opcode::Conversion {
                class: ConversionOp::I32Extend8S,
                uniarg,
            },
            Instruction::I32Extend16S(uniarg) => Opcode::Conversion {
                class: ConversionOp::I32Extend16S,
                uniarg,
            },
            Instruction::I64Extend8S(uniarg) => Opcode::Conversion {
                class: ConversionOp::I64Extend8S,
                uniarg,
            },
            Instruction::I64Extend16S(uniarg) => Opcode::Conversion {
                class: ConversionOp::I64Extend16S,
                uniarg,
            },
            Instruction::I64Extend32S(uniarg) => Opcode::Conversion {
                class: ConversionOp::I64Extend32S,
                uniarg,
            },
        }
    }
}

pub(super) enum RunInstructionTracePre {
    BrIfEqz {
        value: i32,
    },
    BrIfNez {
        value: i32,
    },
    BrTable {
        index: i32,
    },

    Call,
    CallIndirect {
        table_idx: u32,
        type_idx: u32,
        offset: u32,
    },

    SetLocal {
        depth: u32,
        value: ValueInternal,
        vtype: ValueType,
    },
    SetGlobal,
    Load {
        offset: u32,
        raw_address: u32,
        effective_address: Option<u32>, // use option in case of memory out of bound
        vtype: ValueType,
        load_size: MemoryReadSize,
    },
    Store {
        offset: u32,
        raw_address: u32,
        effective_address: Option<u32>,
        value: u64,
        vtype: ValueType,
        store_size: MemoryStoreSize,
        pre_block_value1: Option<u64>,
        pre_block_value2: Option<u64>,
    },

    GrowMemory(i32),

    I32(i32),
    I64(i64),
    I32BinOp {
        left: i32,
        right: i32,
    },

    I64BinOp {
        left: i64,
        right: i64,
    },

    I32Single(i32),
    I64Single(i64),

    I64ExtendI32 {
        value: i32,
        sign: bool,
    },

    UnaryOp {
        operand: u64,
        vtype: VarType,
    },

    Drop,
    Select {
        lhs: u64,
        rhs: u64,
        cond: u64,
    },
}

pub(super) fn run_instruction_pre(
    value_stack: &ValueStack,
    function_context: &FunctionContext,
    instructions: &isa::Instruction,
) -> Option<RunInstructionTracePre> {
    match *instructions {
        isa::Instruction::GetLocal(..) => None,
        isa::Instruction::SetLocal(depth, vtype, ..) => {
            let value = value_stack.top();
            Some(RunInstructionTracePre::SetLocal {
                depth,
                value: *value,
                vtype,
            })
        }
        isa::Instruction::TeeLocal(..) => None,
        isa::Instruction::GetGlobal(..) => None,
        isa::Instruction::SetGlobal(..) => Some(RunInstructionTracePre::SetGlobal),

        isa::Instruction::Br(_) => None,
        isa::Instruction::BrIfEqz(..) => Some(RunInstructionTracePre::BrIfEqz {
            value: <_>::from_value_internal(*value_stack.top()),
        }),
        isa::Instruction::BrIfNez(..) => Some(RunInstructionTracePre::BrIfNez {
            value: <_>::from_value_internal(*value_stack.top()),
        }),
        isa::Instruction::BrTable(..) => Some(RunInstructionTracePre::BrTable {
            index: <_>::from_value_internal(*value_stack.top()),
        }),

        isa::Instruction::Unreachable => None,
        isa::Instruction::Return(..) => None,

        isa::Instruction::Call(..) => Some(RunInstructionTracePre::Call),
        isa::Instruction::CallIndirect(type_idx, uniarg) => {
            let table_idx = DEFAULT_TABLE_INDEX;
            let offset = <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]);

            Some(RunInstructionTracePre::CallIndirect {
                table_idx,
                type_idx,
                offset,
            })
        }

        isa::Instruction::Drop => Some(RunInstructionTracePre::Drop),
        isa::Instruction::Select(vtype, lhs, rhs, cond) => {
            let values = value_from_uniargs(&[cond, rhs, lhs], value_stack);

            Some(RunInstructionTracePre::Select {
                cond: from_value_internal_to_u64_with_typ(VarType::I32, values[0]),
                rhs: from_value_internal_to_u64_with_typ(vtype.into(), values[1]),
                lhs: from_value_internal_to_u64_with_typ(vtype.into(), values[2]),
            })
        }

        isa::Instruction::I32Load(offset, uniarg)
        | isa::Instruction::I32Load8S(offset, uniarg)
        | isa::Instruction::I32Load8U(offset, uniarg)
        | isa::Instruction::I32Load16S(offset, uniarg)
        | isa::Instruction::I32Load16U(offset, uniarg) => {
            let load_size = match *instructions {
                isa::Instruction::I32Load(..) => MemoryReadSize::U32,
                isa::Instruction::I32Load8S(..) => MemoryReadSize::S8,
                isa::Instruction::I32Load8U(..) => MemoryReadSize::U8,
                isa::Instruction::I32Load16S(..) => MemoryReadSize::S16,
                isa::Instruction::I32Load16U(..) => MemoryReadSize::U16,
                _ => unreachable!(),
            };

            let raw_address =
                <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]);
            let address = effective_address(offset, raw_address).ok();

            Some(RunInstructionTracePre::Load {
                offset,
                raw_address,
                effective_address: address,
                vtype: parity_wasm::elements::ValueType::I32,
                load_size,
            })
        }
        isa::Instruction::I64Load(offset, uniarg)
        | isa::Instruction::I64Load8S(offset, uniarg)
        | isa::Instruction::I64Load8U(offset, uniarg)
        | isa::Instruction::I64Load16S(offset, uniarg)
        | isa::Instruction::I64Load16U(offset, uniarg)
        | isa::Instruction::I64Load32S(offset, uniarg)
        | isa::Instruction::I64Load32U(offset, uniarg) => {
            let load_size = match *instructions {
                isa::Instruction::I64Load(..) => MemoryReadSize::I64,
                isa::Instruction::I64Load8S(..) => MemoryReadSize::S8,
                isa::Instruction::I64Load8U(..) => MemoryReadSize::U8,
                isa::Instruction::I64Load16S(..) => MemoryReadSize::S16,
                isa::Instruction::I64Load16U(..) => MemoryReadSize::U16,
                isa::Instruction::I64Load32S(..) => MemoryReadSize::S32,
                isa::Instruction::I64Load32U(..) => MemoryReadSize::U32,
                _ => unreachable!(),
            };
            let raw_address =
                <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]);
            let address = effective_address(offset, raw_address).ok();

            Some(RunInstructionTracePre::Load {
                offset,
                raw_address,
                effective_address: address,
                vtype: parity_wasm::elements::ValueType::I64,
                load_size,
            })
        }
        isa::Instruction::I32Store(offset, pos_uniarg, val_uniarg)
        | isa::Instruction::I32Store8(offset, pos_uniarg, val_uniarg)
        | isa::Instruction::I32Store16(offset, pos_uniarg, val_uniarg) => {
            let store_size = match *instructions {
                isa::Instruction::I32Store8(..) => MemoryStoreSize::Byte8,
                isa::Instruction::I32Store16(..) => MemoryStoreSize::Byte16,
                isa::Instruction::I32Store(..) => MemoryStoreSize::Byte32,
                _ => unreachable!(),
            };

            let values = value_from_uniargs(&[val_uniarg, pos_uniarg], value_stack);

            let value: u32 = <_>::from_value_internal(values[0]);
            let raw_address = <_>::from_value_internal(values[1]);
            let address = effective_address(offset, raw_address).ok();

            let pre_block_value1 = address.map(|address| {
                let mut buf = [0u8; 8];
                function_context
                    .memory
                    .clone()
                    .unwrap()
                    .get_into(address / 8 * 8, &mut buf)
                    .unwrap();
                u64::from_le_bytes(buf)
            });

            let pre_block_value2 = address.and_then(|address| {
                if store_size.byte_size() as u32 + address % 8 > 8 {
                    let mut buf = [0u8; 8];
                    function_context
                        .memory
                        .clone()
                        .unwrap()
                        .get_into((address / 8 + 1) * 8, &mut buf)
                        .unwrap();
                    Some(u64::from_le_bytes(buf))
                } else {
                    None
                }
            });

            Some(RunInstructionTracePre::Store {
                offset,
                raw_address,
                effective_address: address,
                value: value as u64,
                vtype: parity_wasm::elements::ValueType::I32,
                store_size,
                pre_block_value1,
                pre_block_value2,
            })
        }
        isa::Instruction::I64Store(offset, pos_uniarg, val_uniarg)
        | isa::Instruction::I64Store8(offset, pos_uniarg, val_uniarg)
        | isa::Instruction::I64Store16(offset, pos_uniarg, val_uniarg)
        | isa::Instruction::I64Store32(offset, pos_uniarg, val_uniarg) => {
            let store_size = match *instructions {
                isa::Instruction::I64Store(..) => MemoryStoreSize::Byte64,
                isa::Instruction::I64Store8(..) => MemoryStoreSize::Byte8,
                isa::Instruction::I64Store16(..) => MemoryStoreSize::Byte16,
                isa::Instruction::I64Store32(..) => MemoryStoreSize::Byte32,
                _ => unreachable!(),
            };

            let values = value_from_uniargs(&[val_uniarg, pos_uniarg], value_stack);

            let value = <_>::from_value_internal(values[0]);
            let raw_address = <_>::from_value_internal(values[1]);
            let address = effective_address(offset, raw_address).ok();

            let pre_block_value1 = address.map(|address| {
                let mut buf = [0u8; 8];
                function_context
                    .memory
                    .clone()
                    .unwrap()
                    .get_into(address / 8 * 8, &mut buf)
                    .unwrap();
                u64::from_le_bytes(buf)
            });

            let pre_block_value2 = address.and_then(|address| {
                if store_size.byte_size() as u32 + address % 8 > 8 {
                    let mut buf = [0u8; 8];
                    function_context
                        .memory
                        .clone()
                        .unwrap()
                        .get_into((address / 8 + 1) * 8, &mut buf)
                        .unwrap();
                    Some(u64::from_le_bytes(buf))
                } else {
                    None
                }
            });

            Some(RunInstructionTracePre::Store {
                offset,
                raw_address,
                effective_address: address,
                value,
                vtype: parity_wasm::elements::ValueType::I64,
                store_size,
                pre_block_value1,
                pre_block_value2,
            })
        }

        isa::Instruction::CurrentMemory => None,
        isa::Instruction::GrowMemory(uniarg) => Some(RunInstructionTracePre::GrowMemory(
            <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]),
        )),

        isa::Instruction::I32Const(_) => None,
        isa::Instruction::I64Const(_) => None,

        isa::Instruction::I32Eqz(_) => Some(RunInstructionTracePre::I32Single(
            <_>::from_value_internal(*value_stack.pick(1)),
        )),
        isa::Instruction::I64Eqz(_) => Some(RunInstructionTracePre::I64Single(
            <_>::from_value_internal(*value_stack.pick(1)),
        )),

        isa::Instruction::I32Eq(lhs, rhs)
        | isa::Instruction::I32Ne(lhs, rhs)
        | isa::Instruction::I32GtS(lhs, rhs)
        | isa::Instruction::I32GtU(lhs, rhs)
        | isa::Instruction::I32GeS(lhs, rhs)
        | isa::Instruction::I32GeU(lhs, rhs)
        | isa::Instruction::I32LtU(lhs, rhs)
        | isa::Instruction::I32LeU(lhs, rhs)
        | isa::Instruction::I32LtS(lhs, rhs)
        | isa::Instruction::I32LeS(lhs, rhs)
        | isa::Instruction::I32Add(lhs, rhs)
        | isa::Instruction::I32Sub(lhs, rhs)
        | isa::Instruction::I32Mul(lhs, rhs)
        | isa::Instruction::I32DivS(lhs, rhs)
        | isa::Instruction::I32DivU(lhs, rhs)
        | isa::Instruction::I32RemS(lhs, rhs)
        | isa::Instruction::I32RemU(lhs, rhs)
        | isa::Instruction::I32Shl(lhs, rhs)
        | isa::Instruction::I32ShrU(lhs, rhs)
        | isa::Instruction::I32ShrS(lhs, rhs)
        | isa::Instruction::I32And(lhs, rhs)
        | isa::Instruction::I32Or(lhs, rhs)
        | isa::Instruction::I32Xor(lhs, rhs)
        | isa::Instruction::I32Rotl(lhs, rhs)
        | isa::Instruction::I32Rotr(lhs, rhs) => {
            let uniargs = &[rhs, lhs];
            let values = value_from_uniargs(uniargs, value_stack);
            Some(RunInstructionTracePre::I32BinOp {
                left: <_>::from_value_internal(values[1]),
                right: <_>::from_value_internal(values[0]),
            })
        }

        isa::Instruction::I64Eq(lhs, rhs)
        | isa::Instruction::I64Ne(lhs, rhs)
        | isa::Instruction::I64GtS(lhs, rhs)
        | isa::Instruction::I64GtU(lhs, rhs)
        | isa::Instruction::I64GeS(lhs, rhs)
        | isa::Instruction::I64GeU(lhs, rhs)
        | isa::Instruction::I64LtU(lhs, rhs)
        | isa::Instruction::I64LeU(lhs, rhs)
        | isa::Instruction::I64LtS(lhs, rhs)
        | isa::Instruction::I64LeS(lhs, rhs)
        | isa::Instruction::I64Add(lhs, rhs)
        | isa::Instruction::I64Sub(lhs, rhs)
        | isa::Instruction::I64Mul(lhs, rhs)
        | isa::Instruction::I64DivS(lhs, rhs)
        | isa::Instruction::I64DivU(lhs, rhs)
        | isa::Instruction::I64RemS(lhs, rhs)
        | isa::Instruction::I64RemU(lhs, rhs)
        | isa::Instruction::I64Shl(lhs, rhs)
        | isa::Instruction::I64ShrU(lhs, rhs)
        | isa::Instruction::I64ShrS(lhs, rhs)
        | isa::Instruction::I64And(lhs, rhs)
        | isa::Instruction::I64Or(lhs, rhs)
        | isa::Instruction::I64Xor(lhs, rhs)
        | isa::Instruction::I64Rotl(lhs, rhs)
        | isa::Instruction::I64Rotr(lhs, rhs) => {
            let uniargs = &[rhs, lhs];
            let values = value_from_uniargs(uniargs, value_stack);
            Some(RunInstructionTracePre::I64BinOp {
                left: <_>::from_value_internal(values[1]),
                right: <_>::from_value_internal(values[0]),
            })
        }

        isa::Instruction::I32Ctz(..)
        | isa::Instruction::I32Clz(..)
        | isa::Instruction::I32Popcnt(..) => Some(RunInstructionTracePre::UnaryOp {
            operand: from_value_internal_to_u64_with_typ(VarType::I32, *value_stack.pick(1)),
            vtype: VarType::I32,
        }),
        isa::Instruction::I64Ctz(..)
        | isa::Instruction::I64Clz(..)
        | isa::Instruction::I64Popcnt(..) => Some(RunInstructionTracePre::UnaryOp {
            operand: from_value_internal_to_u64_with_typ(VarType::I64, *value_stack.pick(1)),
            vtype: VarType::I64,
        }),

        isa::Instruction::I32WrapI64(uniarg) => Some(RunInstructionTracePre::I64(
            <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]),
        )),
        isa::Instruction::I64ExtendUI32(uniarg) => Some(RunInstructionTracePre::I64ExtendI32 {
            value: <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]),
            sign: false,
        }),
        isa::Instruction::I64ExtendSI32(uniarg) => Some(RunInstructionTracePre::I64ExtendI32 {
            value: <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]),
            sign: true,
        }),
        isa::Instruction::I32Extend8S(uniarg) | isa::Instruction::I32Extend16S(uniarg) => {
            Some(RunInstructionTracePre::I32(<_>::from_value_internal(
                value_from_uniargs(&[uniarg], value_stack)[0],
            )))
        }
        isa::Instruction::I64Extend8S(uniarg)
        | isa::Instruction::I64Extend16S(uniarg)
        | isa::Instruction::I64Extend32S(uniarg) => Some(RunInstructionTracePre::I64(
            <_>::from_value_internal(value_from_uniargs(&[uniarg], value_stack)[0]),
        )),

        _ => {
            println!("{:?}", *instructions);
            unimplemented!()
        }
    }
}

impl TablePlugin {
    pub(super) fn run_instruction_post(
        &self,
        module_ref: &ModuleRef,
        current_event: Option<RunInstructionTracePre>,
        value_stack: &ValueStack,
        context: &FunctionContext,
        instruction: &isa::Instruction,
    ) -> StepInfo {
        match *instruction {
            isa::Instruction::GetLocal(depth, vtype) => StepInfo::GetLocal {
                depth,
                value: from_value_internal_to_u64_with_typ(vtype.into(), *value_stack.top()),
                vtype: vtype.into(),
            },
            isa::Instruction::SetLocal(_, _, uniarg) => {
                if let RunInstructionTracePre::SetLocal {
                    depth,
                    value,
                    vtype,
                } = current_event.unwrap()
                {
                    StepInfo::SetLocal {
                        depth,
                        value: from_value_internal_to_u64_with_typ(vtype.into(), value),
                        vtype: vtype.into(),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::TeeLocal(depth, vtype) => StepInfo::TeeLocal {
                depth,
                value: from_value_internal_to_u64_with_typ(vtype.into(), *value_stack.top()),
                vtype: vtype.into(),
            },
            isa::Instruction::GetGlobal(idx) => {
                let global_ref = context.module().global_by_index(idx).unwrap();
                let is_mutable = global_ref.is_mutable();
                let vtype: VarType = global_ref.value_type().into_elements().into();
                let value = from_value_internal_to_u64_with_typ(
                    vtype,
                    ValueInternal::from(global_ref.get()),
                );

                StepInfo::GetGlobal {
                    idx,
                    vtype,
                    is_mutable,
                    value,
                }
            }
            isa::Instruction::SetGlobal(idx, uniarg) => {
                let global_ref = context.module().global_by_index(idx).unwrap();
                let is_mutable = global_ref.is_mutable();
                let vtype: VarType = global_ref.value_type().into_elements().into();
                let value = from_value_internal_to_u64_with_typ(
                    vtype,
                    ValueInternal::from(global_ref.get()),
                );

                StepInfo::SetGlobal {
                    idx,
                    vtype,
                    is_mutable,
                    value,
                    uniarg,
                }
            }

            isa::Instruction::Br(target) => StepInfo::Br {
                dst_pc: target.dst_pc,
                drop: target.drop_keep.drop,
                keep: if let Keep::Single(t) = target.drop_keep.keep {
                    vec![t.into()]
                } else {
                    vec![]
                },
                keep_values: match target.drop_keep.keep {
                    Keep::Single(t) => vec![from_value_internal_to_u64_with_typ(
                        t.into(),
                        *value_stack.top(),
                    )],
                    Keep::None => vec![],
                },
            },
            isa::Instruction::BrIfEqz(target, ..) => {
                if let RunInstructionTracePre::BrIfEqz { value } = current_event.unwrap() {
                    StepInfo::BrIfEqz {
                        condition: value,
                        dst_pc: target.dst_pc,
                        drop: target.drop_keep.drop,
                        keep: if let Keep::Single(t) = target.drop_keep.keep {
                            vec![t.into()]
                        } else {
                            vec![]
                        },
                        keep_values: match target.drop_keep.keep {
                            Keep::Single(t) => vec![from_value_internal_to_u64_with_typ(
                                t.into(),
                                *value_stack.top(),
                            )],
                            Keep::None => vec![],
                        },
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::BrIfNez(target, ..) => {
                if let RunInstructionTracePre::BrIfNez { value } = current_event.unwrap() {
                    StepInfo::BrIfNez {
                        condition: value,
                        dst_pc: target.dst_pc,
                        drop: target.drop_keep.drop,
                        keep: if let Keep::Single(t) = target.drop_keep.keep {
                            vec![t.into()]
                        } else {
                            vec![]
                        },
                        keep_values: match target.drop_keep.keep {
                            Keep::Single(t) => vec![from_value_internal_to_u64_with_typ(
                                t.into(),
                                *value_stack.top(),
                            )],
                            Keep::None => vec![],
                        },
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::BrTable(targets, ..) => {
                if let RunInstructionTracePre::BrTable { index } = current_event.unwrap() {
                    StepInfo::BrTable {
                        index,
                        dst_pc: targets.get(index as u32).dst_pc,
                        drop: targets.get(index as u32).drop_keep.drop,
                        keep: if let Keep::Single(t) = targets.get(index as u32).drop_keep.keep {
                            vec![t.into()]
                        } else {
                            vec![]
                        },
                        keep_values: match targets.get(index as u32).drop_keep.keep {
                            Keep::Single(t) => vec![from_value_internal_to_u64_with_typ(
                                t.into(),
                                *value_stack.top(),
                            )],
                            Keep::None => vec![],
                        },
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::Return(DropKeep { drop, keep }) => {
                let mut drop_values = vec![];

                for i in 1..=drop {
                    drop_values.push(*value_stack.pick(i as usize));
                }

                StepInfo::Return {
                    drop,
                    keep: if let Keep::Single(t) = keep {
                        vec![t.into()]
                    } else {
                        vec![]
                    },
                    keep_values: match keep {
                        Keep::Single(t) => vec![from_value_internal_to_u64_with_typ(
                            t.into(),
                            *value_stack.top(),
                        )],
                        Keep::None => vec![],
                    },
                }
            }

            isa::Instruction::Drop => {
                if let RunInstructionTracePre::Drop = current_event.unwrap() {
                    StepInfo::Drop
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::Select(vtype, lhs_uniarg, rhs_uniarg, cond_uniarg) => {
                if let RunInstructionTracePre::Select { lhs, rhs, cond } = current_event.unwrap() {
                    StepInfo::Select {
                        lhs,
                        lhs_uniarg,
                        rhs,
                        rhs_uniarg,
                        cond,
                        cond_uniarg,
                        result: from_value_internal_to_u64_with_typ(
                            vtype.into(),
                            *value_stack.top(),
                        ),
                        vtype: vtype.into(),
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::Call(index) => {
                if let RunInstructionTracePre::Call = current_event.unwrap() {
                    let desc = &self.function_table[index as usize];

                    match &desc.ftype {
                        specs::types::FunctionType::WasmFunction => StepInfo::Call { index },
                        specs::types::FunctionType::HostFunction {
                            plugin,
                            function_index: host_function_idx,
                            function_name,
                            op_index_in_plugin,
                        } => {
                            let params_len = desc.signature.params().len();
                            let mut args: Vec<u64> = vec![];
                            let signature: specs::host_function::Signature =
                                desc.signature.clone().into();
                            let params = signature.params.clone();

                            for (i, param) in params.iter().enumerate().take(params_len) {
                                args.push(from_value_internal_to_u64_with_typ(
                                    param.into(),
                                    *value_stack.pick(params_len - i),
                                ));
                            }
                            StepInfo::CallHost {
                                plugin: *plugin,
                                host_function_idx: *host_function_idx,
                                function_name: function_name.clone(),
                                args,
                                ret_val: None,
                                signature,
                                op_index_in_plugin: *op_index_in_plugin,
                            }
                        }
                        specs::types::FunctionType::HostFunctionExternal { op, sig, .. } => {
                            StepInfo::ExternalHostCall {
                                op: *op,
                                value: match sig {
                                    ExternalHostCallSignature::Argument => {
                                        Some(from_value_internal_to_u64_with_typ(
                                            VarType::I64,
                                            *value_stack.top(),
                                        ))
                                    }
                                    ExternalHostCallSignature::Return => None,
                                },
                                sig: *sig,
                            }
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::CallIndirect(_idx, uniarg) => {
                if let RunInstructionTracePre::CallIndirect {
                    table_idx,
                    type_idx,
                    offset,
                } = current_event.unwrap()
                {
                    let table = context
                        .module()
                        .table_by_index(DEFAULT_TABLE_INDEX)
                        .unwrap();
                    let func_ref = table.get(offset).unwrap().unwrap();
                    let func_index = module_ref.func_index_by_func_ref(&func_ref);

                    StepInfo::CallIndirect {
                        table_index: table_idx,
                        type_index: type_idx,
                        offset,
                        func_index,
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I32Load(_offset, uniarg)
            | isa::Instruction::I32Load8U(_offset, uniarg)
            | isa::Instruction::I32Load8S(_offset, uniarg)
            | isa::Instruction::I32Load16U(_offset, uniarg)
            | isa::Instruction::I32Load16S(_offset, uniarg)
            | isa::Instruction::I64Load(_offset, uniarg)
            | isa::Instruction::I64Load8U(_offset, uniarg)
            | isa::Instruction::I64Load8S(_offset, uniarg)
            | isa::Instruction::I64Load16U(_offset, uniarg)
            | isa::Instruction::I64Load16S(_offset, uniarg)
            | isa::Instruction::I64Load32U(_offset, uniarg)
            | isa::Instruction::I64Load32S(_offset, uniarg) => {
                if let RunInstructionTracePre::Load {
                    offset,
                    raw_address,
                    effective_address,
                    vtype,
                    load_size,
                } = current_event.unwrap()
                {
                    let block_value1 = {
                        let mut buf = [0u8; 8];
                        context
                            .memory
                            .clone()
                            .unwrap()
                            .get_into(effective_address.unwrap() / 8 * 8, &mut buf)
                            .unwrap();
                        u64::from_le_bytes(buf)
                    };

                    let block_value2 = if effective_address.unwrap() % 8 + load_size.byte_size() > 8
                    {
                        let mut buf = [0u8; 8];
                        context
                            .memory
                            .clone()
                            .unwrap()
                            .get_into((effective_address.unwrap() / 8 + 1) * 8, &mut buf)
                            .unwrap();
                        u64::from_le_bytes(buf)
                    } else {
                        0
                    };

                    StepInfo::Load {
                        vtype: vtype.into(),
                        load_size,
                        offset,
                        raw_address,
                        effective_address: effective_address.unwrap(),
                        value: from_value_internal_to_u64_with_typ(
                            vtype.into(),
                            *value_stack.top(),
                        ),
                        block_value1,
                        block_value2,
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I32Store(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I32Store8(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I32Store16(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I64Store(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I64Store8(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I64Store16(_, pos_uniarg, val_uniarg)
            | isa::Instruction::I64Store32(_, pos_uniarg, val_uniarg) => {
                if let RunInstructionTracePre::Store {
                    offset,
                    raw_address,
                    effective_address,
                    value,
                    vtype,
                    store_size,
                    pre_block_value1,
                    pre_block_value2,
                } = current_event.unwrap()
                {
                    let updated_block_value1 = {
                        let mut buf = [0u8; 8];
                        context
                            .memory
                            .clone()
                            .unwrap()
                            .get_into(effective_address.unwrap() / 8 * 8, &mut buf)
                            .unwrap();
                        u64::from_le_bytes(buf)
                    };

                    let updated_block_value2 =
                        if effective_address.unwrap() % 8 + store_size.byte_size() as u32 > 8 {
                            let mut buf = [0u8; 8];
                            context
                                .memory
                                .clone()
                                .unwrap()
                                .get_into((effective_address.unwrap() / 8 + 1) * 8, &mut buf)
                                .unwrap();
                            u64::from_le_bytes(buf)
                        } else {
                            0
                        };

                    StepInfo::Store {
                        vtype: vtype.into(),
                        store_size,
                        offset,
                        raw_address,
                        effective_address: effective_address.unwrap(),
                        value,
                        pre_block_value1: pre_block_value1.unwrap(),
                        pre_block_value2: pre_block_value2.unwrap_or(0u64),
                        updated_block_value1,
                        updated_block_value2,
                        pos_uniarg,
                        val_uniarg,
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::CurrentMemory => StepInfo::MemorySize,
            isa::Instruction::GrowMemory(uniarg) => {
                if let RunInstructionTracePre::GrowMemory(grow_size) = current_event.unwrap() {
                    StepInfo::MemoryGrow {
                        grow_size,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I32Const(value) => StepInfo::I32Const { value },
            isa::Instruction::I64Const(value) => StepInfo::I64Const { value },

            isa::Instruction::I32Eqz(..) => {
                if let RunInstructionTracePre::I32Single(value) = current_event.unwrap() {
                    StepInfo::Test {
                        vtype: VarType::I32,
                        value: value as u32 as u64,
                        result: <_>::from_value_internal(*value_stack.top()),
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I64Eqz(..) => {
                if let RunInstructionTracePre::I64Single(value) = current_event.unwrap() {
                    StepInfo::Test {
                        vtype: VarType::I64,
                        value: value as u64,
                        result: <_>::from_value_internal(*value_stack.top()),
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I32Add(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Sub(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Mul(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32DivU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32RemU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32DivS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32RemS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32And(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Or(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Xor(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Shl(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32ShrU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32ShrS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Rotl(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Rotr(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Eq(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32Ne(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32GtS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32GtU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32GeS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32GeU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32LtS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32LtU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32LeS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I32LeU(lhs_uniarg, rhs_uniarg) => {
                if let RunInstructionTracePre::I32BinOp { left, right } = current_event.unwrap() {
                    StepInfo::I32BinOp {
                        class: BinaryOp::from(instruction),
                        left,
                        lhs_uniarg,
                        right,
                        rhs_uniarg,
                        value: <_>::from_value_internal(*value_stack.top()),
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I64Add(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Sub(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Mul(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64DivU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64RemU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64DivS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64RemS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64And(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Or(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Xor(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Shl(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64ShrU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64ShrS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Rotl(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Rotr(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Eq(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64Ne(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64GtS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64GtU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64GeS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64GeU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64LtS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64LtU(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64LeS(lhs_uniarg, rhs_uniarg)
            | isa::Instruction::I64LeU(lhs_uniarg, rhs_uniarg) => {
                if let RunInstructionTracePre::I64BinOp { left, right } = current_event.unwrap() {
                    let class: BinaryOp = BinaryOp::from(instruction).into();
                    let value_type = if class.is_rel_op() {
                        VarType::I32
                    } else {
                        VarType::I64
                    };

                    StepInfo::I64BinOp {
                        class,
                        left,
                        lhs_uniarg,
                        right,
                        rhs_uniarg,
                        value: <_>::from_value_internal(*value_stack.top()),
                        value_type,
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I32Ctz(..)
            | isa::Instruction::I32Clz(..)
            | isa::Instruction::I32Popcnt(..)
            | isa::Instruction::I64Ctz(..)
            | isa::Instruction::I64Clz(..)
            | isa::Instruction::I64Popcnt(..) => {
                if let RunInstructionTracePre::UnaryOp { operand, vtype } = current_event.unwrap() {
                    StepInfo::UnaryOp {
                        class: UnaryOp::from(instruction),
                        vtype,
                        operand,
                        result: from_value_internal_to_u64_with_typ(vtype, *value_stack.top()),
                    }
                } else {
                    unreachable!()
                }
            }

            isa::Instruction::I32WrapI64(uniarg) => {
                if let RunInstructionTracePre::I64(value) = current_event.unwrap() {
                    StepInfo::I32WrapI64 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I64ExtendSI32(uniarg) | isa::Instruction::I64ExtendUI32(uniarg) => {
                if let RunInstructionTracePre::I64ExtendI32 { value, sign } = current_event.unwrap()
                {
                    StepInfo::I64ExtendI32 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        sign,
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I32Extend8S(uniarg) => {
                if let RunInstructionTracePre::I32(value) = current_event.unwrap() {
                    StepInfo::I32SignExtendI8 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I32Extend16S(uniarg) => {
                if let RunInstructionTracePre::I32(value) = current_event.unwrap() {
                    StepInfo::I32SignExtendI16 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I64Extend8S(uniarg) => {
                if let RunInstructionTracePre::I64(value) = current_event.unwrap() {
                    StepInfo::I64SignExtendI8 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I64Extend16S(uniarg) => {
                if let RunInstructionTracePre::I64(value) = current_event.unwrap() {
                    StepInfo::I64SignExtendI16 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }
            isa::Instruction::I64Extend32S(uniarg) => {
                if let RunInstructionTracePre::I64(value) = current_event.unwrap() {
                    StepInfo::I64SignExtendI32 {
                        value,
                        result: <_>::from_value_internal(*value_stack.top()),
                        uniarg,
                    }
                } else {
                    unreachable!()
                }
            }

            _ => {
                println!("{:?}", instruction);
                unimplemented!()
            }
        }
    }
}
