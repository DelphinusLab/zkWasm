use std::collections::HashMap;

use halo2_proofs::arithmetic::FieldExt;
use specs::etable::EventTableEntry;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::itable::UniArg;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MemoryTableEntry;
use specs::mtable::VarType;
use specs::step::StepInfo;

use crate::foreign::context::ContextOutput;

use self::host::ForeignStatics;
use self::wasmi_interpreter::WasmiRuntime;

pub mod host;
pub mod monitor;
pub mod state;
pub mod wasmi_interpreter;

pub struct CompiledImage<I> {
    pub entry: String,
    // pub tables: CompilationTable,
    pub instance: I,
    // pub tracer: Rc<RefCell<T>>,
}

pub struct ExecutionResult<R> {
    // pub tables: Tables,
    pub result: Option<R>,
    pub public_inputs_and_outputs: Vec<u64>,
    pub host_statics: HashMap<String, ForeignStatics>,
    pub guest_statics: usize, // total instructions used in guest circuits
    pub outputs: Vec<u64>,
    pub context_outputs: ContextOutput,
}

impl<R> ExecutionResult<R> {
    pub fn public_inputs_and_outputs<F: FieldExt>(&self) -> Vec<F> {
        self.public_inputs_and_outputs
            .iter()
            .map(|v| F::from(*v))
            .collect()
    }
}

// TODO: use feature
pub type WasmInterpreter = WasmiRuntime;

pub fn memory_event_of_step(event: &EventTableEntry) -> Vec<MemoryTableEntry> {
    let eid = event.eid;
    let sp_before_execution = event.sp;

    match &event.step_info {
        StepInfo::Br {
            drop,
            keep,
            keep_values,
            ..
        } => {
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let mut sp = sp_before_execution + 1;
            let mut ops = vec![];

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp += 1;
                }
            }

            sp += drop;
            sp -= 1;

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Write,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp -= 1;
                }
            }

            ops
        }
        StepInfo::BrIfEqz {
            condition,
            drop,
            keep,
            keep_values,
            uniarg,
            ..
        } => {
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let (mut sp, mut ops) = _mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(VarType::I32, *uniarg, *condition as u32 as u64)],
                None,
            );

            if *condition == 0 {
                sp += 1;

                {
                    for i in 0..keep.len() {
                        ops.push(MemoryTableEntry {
                            eid,
                            offset: sp,
                            ltype: LocationType::Stack,
                            atype: AccessType::Read,
                            vtype: keep[i].into(),
                            is_mutable: true,
                            value: keep_values[i],
                        });

                        sp += 1;
                    }
                }

                sp += drop;
                sp -= 1;

                {
                    for i in 0..keep.len() {
                        ops.push(MemoryTableEntry {
                            eid,
                            offset: sp,
                            ltype: LocationType::Stack,
                            atype: AccessType::Write,
                            vtype: keep[i].into(),
                            is_mutable: true,
                            value: keep_values[i],
                        });

                        sp -= 1;
                    }
                }
            }

            ops
        }
        StepInfo::BrIfNez {
            condition,
            drop,
            keep,
            keep_values,
            uniarg,
            ..
        } => {
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let (mut sp, mut ops) = _mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(VarType::I32, *uniarg, *condition as u32 as u64)],
                None,
            );

            if *condition != 0 {
                sp += 1;

                {
                    for i in 0..keep.len() {
                        ops.push(MemoryTableEntry {
                            eid,
                            offset: sp,
                            ltype: LocationType::Stack,
                            atype: AccessType::Read,
                            vtype: keep[i].into(),
                            is_mutable: true,
                            value: keep_values[i],
                        });

                        sp += 1;
                    }
                }

                sp += drop;
                sp -= 1;

                {
                    for i in 0..keep.len() {
                        ops.push(MemoryTableEntry {
                            eid,
                            offset: sp,
                            ltype: LocationType::Stack,
                            atype: AccessType::Write,
                            vtype: keep[i].into(),
                            is_mutable: true,
                            value: keep_values[i],
                        });

                        sp -= 1;
                    }
                }
            }

            ops
        }
        StepInfo::BrTable {
            index,
            drop,
            keep,
            keep_values,
            uniarg,
            ..
        } => {
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let (mut sp, mut ops) = _mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(VarType::I32, *uniarg, *index as u32 as u64)],
                None,
            );

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp += 1;
                }
            }

            sp += drop;
            sp -= 1;

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Write,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp -= 1;
                }
            }

            ops
        }
        StepInfo::Return {
            drop,
            keep,
            keep_values,
        } => {
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let mut sp = sp_before_execution + 1;
            let mut ops = vec![];

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp += 1;
                }
            }

            sp += drop;
            sp -= 1;

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Write,
                        vtype: keep[i].into(),
                        is_mutable: true,
                        value: keep_values[i],
                    });

                    sp -= 1;
                }
            }

            ops
        }
        StepInfo::Drop { .. } => vec![],
        StepInfo::Select {
            lhs,
            lhs_uniarg,
            rhs,
            rhs_uniarg,
            cond,
            cond_uniarg,
            result,
            vtype,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[
                (*vtype, *lhs_uniarg, *lhs as u64),
                (*vtype, *rhs_uniarg, *rhs as u64),
                (VarType::I32, *cond_uniarg, *cond as u64),
            ],
            Some((*vtype, *result as u64)),
        ),
        StepInfo::Call { index: _ } => {
            vec![]
        }
        StepInfo::CallIndirect { offset, uniarg, .. } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I32, *uniarg, *offset as u64)],
            None,
        ),
        StepInfo::CallHost {
            args,
            ret_val,
            signature,
            ..
        } => {
            let mut mops = vec![];
            let mut sp = sp_before_execution;

            for (i, (ty, val)) in signature.params.iter().zip(args.iter()).enumerate() {
                mops.push(MemoryTableEntry {
                    eid,
                    offset: sp_before_execution + args.len() as u32 - i as u32,
                    ltype: LocationType::Stack,
                    atype: AccessType::Read,
                    vtype: (*ty).into(),
                    is_mutable: true,
                    value: *val,
                });
            }

            sp += args.len() as u32;

            if let Some(ty) = signature.return_type {
                mops.push(MemoryTableEntry {
                    eid,
                    offset: sp,
                    ltype: LocationType::Stack,
                    atype: AccessType::Write,
                    vtype: ty.into(),
                    is_mutable: true,
                    value: ret_val.unwrap(),
                });
            }

            mops
        }
        StepInfo::ExternalHostCall { value, sig, .. } => match sig {
            ExternalHostCallSignature::Argument => {
                let stack_read = MemoryTableEntry {
                    eid,
                    offset: sp_before_execution + 1,
                    ltype: LocationType::Stack,
                    atype: AccessType::Read,
                    vtype: VarType::I64,
                    is_mutable: true,
                    value: value.unwrap(),
                };

                vec![stack_read]
            }
            ExternalHostCallSignature::Return => {
                let stack_write = MemoryTableEntry {
                    eid,
                    offset: sp_before_execution,
                    ltype: LocationType::Stack,
                    atype: AccessType::Write,
                    vtype: VarType::I64,
                    is_mutable: true,
                    value: value.unwrap(),
                };

                vec![stack_write]
            }
        },

        StepInfo::GetLocal {
            vtype,
            depth,
            value,
        } => {
            let read = MemoryTableEntry {
                eid,
                offset: sp_before_execution + depth,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            };

            let write = MemoryTableEntry {
                eid,
                offset: sp_before_execution,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            };
            vec![read, write]
        }
        StepInfo::SetLocal {
            vtype,
            depth,
            value,
            uniarg,
        } => {
            let (sp, mut ops) = _mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(*vtype, *uniarg, *value)],
                None,
            );

            ops.push(MemoryTableEntry {
                eid,
                offset: sp + depth,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            });

            ops
        }
        StepInfo::TeeLocal {
            vtype,
            depth,
            value,
        } => {
            let read = MemoryTableEntry {
                eid,
                offset: sp_before_execution + 1,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            };

            let write = MemoryTableEntry {
                eid,
                offset: sp_before_execution + depth,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            };

            vec![read, write]
        }

        StepInfo::GetGlobal {
            idx,
            vtype,
            is_mutable,
            value,
            ..
        } => {
            let global_get = MemoryTableEntry {
                eid,
                offset: *idx,
                ltype: LocationType::Global,
                atype: AccessType::Read,
                vtype: *vtype,
                is_mutable: *is_mutable,
                value: *value,
            };

            let stack_write = MemoryTableEntry {
                eid,
                offset: sp_before_execution,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                is_mutable: true,
                value: *value,
            };

            vec![global_get, stack_write]
        }
        StepInfo::SetGlobal {
            idx,
            vtype,
            is_mutable,
            value,
            uniarg,
        } => {
            let mut ops = mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(*vtype, *uniarg, *value)],
                None,
            );

            let global_set = MemoryTableEntry {
                eid,
                offset: *idx,
                ltype: LocationType::Global,
                atype: AccessType::Write,
                vtype: *vtype,
                is_mutable: *is_mutable,
                value: *value,
            };

            ops.push(global_set);

            ops
        }

        StepInfo::Load {
            vtype,
            load_size,
            raw_address,
            effective_address,
            value,
            block_value1,
            block_value2,
            uniarg,
            ..
        } => {
            let mut ops = mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[(VarType::I32, *uniarg, *raw_address as u64)],
                Some((*vtype, *value)),
            );

            let write_result = ops.pop().unwrap();

            // load first block
            ops.push(MemoryTableEntry {
                eid,
                offset: (*effective_address) / 8,
                ltype: LocationType::Heap,
                atype: AccessType::Read,
                // Load u64 from address which align with 8
                vtype: VarType::I64,
                is_mutable: true,
                // The value will be used to lookup within imtable, hence block_value is given here
                value: *block_value1,
            });

            // load second block if it is cross access
            if *effective_address % 8 + load_size.byte_size() > 8 {
                ops.push(MemoryTableEntry {
                    eid,
                    offset: effective_address / 8 + 1,
                    ltype: LocationType::Heap,
                    atype: AccessType::Read,
                    // Load u64 from address which align with 8
                    vtype: VarType::I64,
                    is_mutable: true,
                    // The value will be used to lookup within imtable, hence block_value is given here
                    value: *block_value2,
                })
            }

            ops.push(write_result);

            ops
        }
        StepInfo::Store {
            vtype,
            store_size,
            raw_address,
            effective_address,
            value,
            pre_block_value1,
            updated_block_value1,
            pre_block_value2,
            updated_block_value2,
            val_uniarg,
            pos_uniarg,
            ..
        } => {
            let mut ops = mem_ops_from_stack_only_step(
                sp_before_execution,
                eid,
                &[
                    (VarType::I32, *pos_uniarg, *raw_address as u64),
                    (*vtype, *val_uniarg, *value),
                ],
                None,
            );

            // load first block
            ops.push(MemoryTableEntry {
                eid,
                offset: effective_address / 8,
                ltype: LocationType::Heap,
                atype: AccessType::Read,
                // Load u64 from address which align with 8
                vtype: VarType::I64,
                is_mutable: true,
                // The value will be used to lookup within imtable, hence block_value is given here
                value: *pre_block_value1,
            });

            // write first block
            ops.push(MemoryTableEntry {
                eid,
                offset: effective_address / 8,
                ltype: LocationType::Heap,
                atype: AccessType::Write,
                // Load u64 from address which align with 8
                vtype: VarType::I64,
                is_mutable: true,
                // The value will be used to lookup within imtable, hence block_value is given here
                value: *updated_block_value1,
            });

            if *effective_address % 8 + store_size.byte_size() as u32 > 8 {
                // load second block if cross
                ops.push(MemoryTableEntry {
                    eid,
                    offset: effective_address / 8 + 1,
                    ltype: LocationType::Heap,
                    atype: AccessType::Read,
                    // Load u64 from address which align with 8
                    vtype: VarType::I64,
                    is_mutable: true,
                    // The value will be used to lookup within imtable, hence block_value is given here
                    value: *pre_block_value2,
                });

                // write second block if cross
                ops.push(MemoryTableEntry {
                    eid,
                    offset: effective_address / 8 + 1,
                    ltype: LocationType::Heap,
                    atype: AccessType::Write,
                    // Load u64 from address which align with 8
                    vtype: VarType::I64,
                    is_mutable: true,
                    // The value will be used to lookup within imtable, hence block_value is given here
                    value: *updated_block_value2,
                });
            }

            ops
        }

        StepInfo::MemorySize => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            VarType::I32,
            VarType::I32,
            &[],
            &[event.allocated_memory_pages as u64],
        ),
        StepInfo::MemoryGrow {
            grow_size,
            result,
            uniarg,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I32, *uniarg, *grow_size as u32 as u64)],
            Some((VarType::I32, *result as u32 as u64)),
        ),

        StepInfo::I32Const { value } => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            VarType::I32,
            VarType::I32,
            &[],
            &[*value as u32 as u64],
        ),
        StepInfo::I32BinOp {
            left,
            right,
            value,
            lhs_uniarg,
            rhs_uniarg,
            ..
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[
                (VarType::I32, *lhs_uniarg, *left as u32 as u64),
                (VarType::I32, *rhs_uniarg, *right as u32 as u64),
            ],
            Some((VarType::I32, *value as u32 as u64)),
        ),
        StepInfo::I64BinOp {
            left,
            right,
            value,
            value_type,
            lhs_uniarg,
            rhs_uniarg,
            ..
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[
                (VarType::I64, *lhs_uniarg, *left as u64),
                (VarType::I64, *rhs_uniarg, *right as u64),
            ],
            Some((*value_type, *value as u64)),
        ),

        StepInfo::I64Const { value } => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            VarType::I64,
            VarType::I64,
            &[],
            &[*value as u64],
        ),
        StepInfo::UnaryOp {
            vtype,
            operand,
            result,
            uniarg,
            ..
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(*vtype, *uniarg, *operand)],
            Some((*vtype, *result)),
        ),

        StepInfo::Test {
            vtype,
            value,
            result,
            uniarg,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(*vtype, *uniarg, *value)],
            Some((VarType::I32, *result as u32 as u64)),
        ),

        StepInfo::I32WrapI64 {
            value,
            result,
            uniarg,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I64, *uniarg, *value as u64)],
            Some((VarType::I32, *result as u32 as u64)),
        ),
        StepInfo::I64ExtendI32 {
            value,
            result,
            uniarg,
            ..
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I32, *uniarg, *value as u32 as u64)],
            Some((VarType::I64, *result as u64)),
        ),
        StepInfo::I32SignExtendI8 {
            value,
            result,
            uniarg,
        }
        | StepInfo::I32SignExtendI16 {
            value,
            result,
            uniarg,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I32, *uniarg, *value as u32 as u64)],
            Some((VarType::I32, *result as u32 as u64)),
        ),
        StepInfo::I64SignExtendI8 {
            value,
            result,
            uniarg,
        }
        | StepInfo::I64SignExtendI16 {
            value,
            result,
            uniarg,
        }
        | StepInfo::I64SignExtendI32 {
            value,
            result,
            uniarg,
        } => mem_ops_from_stack_only_step(
            sp_before_execution,
            eid,
            &[(VarType::I64, *uniarg, *value as u64)],
            Some((VarType::I64, *result as u64)),
        ),
    }
}

fn _mem_ops_from_stack_only_step(
    sp_before_execution: u32,
    eid: u32,
    inputs: &[(VarType, UniArg, u64)],
    output: Option<(VarType, u64)>,
) -> (u32, Vec<MemoryTableEntry>) {
    let (sp, mut ops) = inputs.iter().rev().fold(
        (sp_before_execution, vec![]),
        |(mut sp, mut ops), (vtype, arg, input)| {
            match arg {
                UniArg::Pop => {
                    sp += 1;

                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: *vtype,
                        is_mutable: true,
                        value: *input,
                    });
                }
                UniArg::Stack(depth) => {
                    ops.push(MemoryTableEntry {
                        eid,
                        offset: sp + *depth as u32,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: *vtype,
                        is_mutable: true,
                        value: *input,
                    });
                }
                UniArg::IConst(_) => (),
            };

            (sp, ops)
        },
    );

    let sp = if let Some((vtype, value)) = output {
        ops.push(MemoryTableEntry {
            eid,
            offset: sp,
            ltype: LocationType::Stack,
            atype: AccessType::Write,
            vtype,
            is_mutable: true,
            value,
        });

        sp - 1
    } else {
        sp
    };

    (sp, ops)
}

fn mem_ops_from_stack_only_step(
    sp_before_execution: u32,
    eid: u32,
    inputs: &[(VarType, UniArg, u64)],
    output: Option<(VarType, u64)>,
) -> Vec<MemoryTableEntry> {
    _mem_ops_from_stack_only_step(sp_before_execution, eid, inputs, output).1
}

pub(crate) fn mem_op_from_stack_only_step(
    sp_before_execution: u32,
    eid: u32,
    inputs_type: VarType,
    outputs_type: VarType,
    pop_value: &[u64],
    push_value: &[u64],
) -> Vec<MemoryTableEntry> {
    let mut mem_op = vec![];
    let mut sp = sp_before_execution;

    for value in pop_value {
        mem_op.push(MemoryTableEntry {
            eid,
            offset: sp + 1,
            ltype: LocationType::Stack,
            atype: AccessType::Read,
            vtype: inputs_type,
            is_mutable: true,
            value: *value,
        });
        sp += 1;
    }

    for value in push_value {
        mem_op.push(MemoryTableEntry {
            eid,
            offset: sp,
            ltype: LocationType::Stack,
            atype: AccessType::Write,
            vtype: outputs_type,
            is_mutable: true,
            value: *value,
        });
        sp -= 1;
    }

    mem_op
}
