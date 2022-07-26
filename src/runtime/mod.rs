use specs::{
    etable::EventTableEntry,
    mtable::{AccessType, LocationType, MemoryTableEntry, VarType},
    step::StepInfo,
    types::{CompileError, ExecutionError, Value},
    CompileTable, ExecutionTable,
};

use self::wasmi_interpreter::WasmiRuntime;

pub mod wasmi_interpreter;

pub struct CompileOutcome<M> {
    pub textual_repr: String,
    pub module: M,
    pub tables: CompileTable,
}

pub struct ExecutionOutcome {
    pub tables: ExecutionTable,
}

pub trait WasmRuntime {
    type Module;

    fn new() -> Self;
    fn compile(&self, textual_repr: &str) -> Result<CompileOutcome<Self::Module>, CompileError>;
    fn run(
        &self,
        compile_outcome: &CompileOutcome<Self::Module>,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<ExecutionOutcome, ExecutionError>;
}

// TODO: use feature
pub type WasmInterpreter = WasmiRuntime;

pub fn memory_event_of_step(event: &EventTableEntry, emid: &mut u64) -> Vec<MemoryTableEntry> {
    let eid = event.eid;
    let sp_before_execution = event.sp;

    match &event.step_info {
        StepInfo::BrIfNez {
            condition,
            drop,
            keep,
            drop_values,
            keep_values,
            ..
        } => {
            assert_eq!(*drop as usize, drop_values.len());
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let mut sp = sp_before_execution + 1;

            let mut ops = vec![MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: VarType::I32,
                value: *condition as u32 as u64,
            }];

            sp = sp + 1;
            *emid = (*emid).checked_add(1).unwrap();

            if *condition == 0 {
                return ops;
            }

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        emid: *emid,
                        mmid: 0,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: keep[i].into(),
                        value: keep_values[i],
                    });

                    sp = sp + 1;
                    *emid = (*emid).checked_add(1).unwrap();
                }
            }

            sp = sp + ((*drop) as u64);
            sp -= 1;

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        emid: *emid,
                        mmid: 0,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Write,
                        vtype: keep[i].into(),
                        value: keep_values[i],
                    });

                    sp = sp - 1;
                    *emid = (*emid).checked_add(1).unwrap();
                }
            }

            ops
        }
        StepInfo::Return {
            drop,
            keep,
            drop_values,
            keep_values,
        } => {
            assert_eq!(*drop as usize, drop_values.len());
            assert_eq!(keep.len(), keep_values.len());
            assert!(keep.len() <= 1);

            let mut sp = sp_before_execution + 1;
            let mut ops = vec![];

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        emid: *emid,
                        mmid: 0,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Read,
                        vtype: keep[i].into(),
                        value: keep_values[i],
                    });

                    sp = sp + 1;
                    *emid = (*emid).checked_add(1).unwrap();
                }
            }

            sp = sp + ((*drop) as u64);
            sp -= 1;

            {
                for i in 0..keep.len() {
                    ops.push(MemoryTableEntry {
                        eid,
                        emid: *emid,
                        mmid: 0,
                        offset: sp,
                        ltype: LocationType::Stack,
                        atype: AccessType::Write,
                        vtype: keep[i].into(),
                        value: keep_values[i],
                    });

                    sp = sp - 1;
                    *emid = (*emid).checked_add(1).unwrap();
                }
            }

            ops
        }
        StepInfo::Drop { .. } => vec![],
        StepInfo::Call { index: _ } => {
            vec![]
        }
        StepInfo::GetLocal {
            vtype,
            depth,
            value,
        } => {
            let read = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution + *depth as u64,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: *vtype,
                value: *value,
            };
            *emid = (*emid).checked_add(1).unwrap();

            let write = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                value: *value,
            };
            *emid = (*emid).checked_add(1).unwrap();
            vec![read, write]
        }
        StepInfo::TeeLocal {
            vtype,
            depth,
            value,
        } => {
            let read = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution + 1 as u64,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: *vtype,
                value: *value,
            };
            *emid = (*emid).checked_add(1).unwrap();

            let write = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution + *depth as u64,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                value: *value,
            };
            *emid = (*emid).checked_add(1).unwrap();
            vec![read, write]
        }

        StepInfo::Load {
            vtype,
            offset: _offset,
            raw_address,
            effective_address,
            value,
            block_value,
            mmid,
        } => {
            let load_address_from_stack = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution + 1 as u64,
                ltype: LocationType::Stack,
                atype: AccessType::Read,
                vtype: VarType::I32,
                value: *raw_address as u64,
            };
            *emid = (*emid).checked_add(1).unwrap();

            let load_value = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: *mmid,
                offset: ((*effective_address) / 8) as u64,
                ltype: LocationType::Heap,
                atype: AccessType::Read,
                // Load u64 from address which align with 8
                vtype: VarType::U64,
                // The value will be used to lookup within imtable, hence block_value is given here
                value: *block_value,
            };
            *emid = (*emid).checked_add(1).unwrap();

            let push_value = MemoryTableEntry {
                eid,
                emid: *emid,
                mmid: 0,
                offset: sp_before_execution,
                ltype: LocationType::Stack,
                atype: AccessType::Write,
                vtype: *vtype,
                value: *value,
            };
            *emid = (*emid).checked_add(1).unwrap();

            vec![load_address_from_stack, load_value, push_value]
        }
        StepInfo::Store { .. } => {
            todo!()
        }

        StepInfo::I32Const { value } => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            emid,
            VarType::I32,
            VarType::I32,
            &[],
            &[*value as u32 as u64],
        ),
        StepInfo::I32BinOp {
            left, right, value, ..
        }
        | StepInfo::I32BinBitOp {
            left, right, value, ..
        } => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            emid,
            VarType::I32,
            VarType::I32,
            &[*right as u32 as u64, *left as u32 as u64],
            &[*value as u32 as u64],
        ),
        StepInfo::I32Comp {
            left, right, value, ..
        } => mem_op_from_stack_only_step(
            sp_before_execution,
            eid,
            emid,
            VarType::I32,
            VarType::I32,
            &[*right as u32 as u64, *left as u32 as u64],
            &[*value as u32 as u64],
        ),
    }
}

pub(crate) fn mem_op_from_stack_only_step(
    sp_before_execution: u64,
    eid: u64,
    emid: &mut u64,
    inputs_type: VarType,
    outputs_type: VarType,
    pop_value: &[u64],
    push_value: &[u64],
) -> Vec<MemoryTableEntry> {
    let mut mem_op = vec![];
    let mut sp = sp_before_execution;

    for i in 0..pop_value.len() {
        mem_op.push(MemoryTableEntry {
            eid,
            emid: *emid,
            mmid: 0,
            offset: sp + 1,
            ltype: LocationType::Stack,
            atype: AccessType::Read,
            vtype: inputs_type,
            value: pop_value[i],
        });
        *emid = (*emid).checked_add(1).unwrap();
        sp = sp + 1;
    }

    for i in 0..push_value.len() {
        mem_op.push(MemoryTableEntry {
            eid,
            emid: *emid,
            mmid: 0,
            offset: sp,
            ltype: LocationType::Stack,
            atype: AccessType::Write,
            vtype: outputs_type,
            value: push_value[i],
        });
        sp = sp - 1;
        *emid = (*emid).checked_add(1).unwrap();
    }

    mem_op
}
