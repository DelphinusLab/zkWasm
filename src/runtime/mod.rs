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

pub fn memory_event_of_step(event: &EventTableEntry) -> Vec<MemoryTableEntry> {
    let eid = event.eid;
    let mmid = event.inst.mmid.into();

    match &event.step_info {
        StepInfo::BrIfNez { value, dst_pc: _ } => mem_op_from_stack_only_step(
            eid,
            mmid,
            VarType::I32,
            VarType::I32,
            &[*value as u64],
            &[],
        ),
        StepInfo::Return {
            drop,
            keep,
            drop_values,
            keep_values,
        } => {
            assert_eq!(*drop as usize, drop_values.len());
            assert_eq!(*keep as usize, keep_values.len());
            mem_op_from_stack_only_step(
                eid,
                mmid,
                VarType::I32,
                VarType::I32,
                drop_values.iter().map(|value| *value).collect::<Vec<_>>()[..]
                    .try_into()
                    .unwrap(),
                keep_values.iter().map(|value| *value).collect::<Vec<_>>()[..]
                    .try_into()
                    .unwrap(),
            )
        }
        StepInfo::Drop { value } => {
            mem_op_from_stack_only_step(eid, mmid, VarType::I32, VarType::I32, &[*value], &[])
        }
        StepInfo::Call { index: _ } => {
            vec![]
        }
        StepInfo::GetLocal { vtype, depth, value } => {
            vec![
                MemoryTableEntry {
                    eid,
                    // FIXME: emid is small memory id of eid,
                    // e.g. an opcode get a value front stack top and change it,
                    // its event has two memory ops on the same memory address,
                    // we should have use emid to seq the r/w op, it is an incremental value starting from 1
                    emid: 1,
                    mmid,
                    offset: *depth as u64,
                    ltype: LocationType::Stack,
                    atype: AccessType::Read,
                    // FIXME: use correct type
                    vtype: VarType::I32,
                    value: *value,
                },
                MemoryTableEntry {
                    eid,
                    // FIXME: same to above
                    emid: 1,
                    mmid: mmid.into(),
                    offset: 0,
                    ltype: LocationType::Stack,
                    atype: AccessType::Write,
                    // FIXME: use correct type
                    vtype: VarType::I32,
                    value: *value,
                },
            ]
        }
        StepInfo::I32Const { value } => mem_op_from_stack_only_step(
            eid,
            mmid,
            VarType::I32,
            VarType::I32,
            &[],
            &[*value as u64],
        ),
        StepInfo::I32BinOp { left, right, value } => mem_op_from_stack_only_step(
            eid,
            mmid,
            VarType::I32,
            VarType::I32,
            &[*right as u64, *left as u64],
            &[*value as u64],
        ),
        StepInfo::I32Comp { left, right, value } => mem_op_from_stack_only_step(
            eid,
            mmid,
            VarType::I32,
            VarType::I32,
            &[*right as u64, *left as u64],
            &[*value as u64],
        ),
    }
}

pub(crate) fn mem_op_from_stack_only_step(
    eid: u64,
    mmid: u64,
    inputs_type: VarType,
    outputs_type: VarType,
    pop_value: &[u64],
    push_value: &[u64],
) -> Vec<MemoryTableEntry> {
    let mut mem_op = vec![];

    for i in 0..pop_value.len() {
        mem_op.push(MemoryTableEntry {
            eid,
            // FIXME: same to above
            emid: 1,
            mmid,
            offset: i as u64,
            ltype: LocationType::Stack,
            atype: AccessType::Read,
            vtype: inputs_type,
            value: pop_value[i],
        });
    }

    for i in 0..push_value.len() {
        mem_op.push(MemoryTableEntry {
            eid,
            // FIXME: same to above
            emid: 1,
            mmid,
            offset: i as u64,
            ltype: LocationType::Stack,
            atype: AccessType::Write,
            vtype: outputs_type,
            value: push_value[i],
        });
    }

    mem_op
}
