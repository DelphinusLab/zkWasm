use wasmi::tracer::etable::RunInstructionTraceStep;

use crate::{
    etable::Event,
    mtable::{AccessType, LocationType, MemoryEvent, VarType},
};

use self::stack_only::mem_op_from_stack_only_step;

pub(crate) mod stack_only;

pub fn memory_event_of_step(event: &Event) -> Vec<MemoryEvent> {
    let eid = event.eid;
    let mmid = event.inst.mmid.into();

    match event.step_info {
        RunInstructionTraceStep::BrIfNez {
            value: _,
            dst_pc: _,
        } => todo!(),
        RunInstructionTraceStep::Return { drop: _, keep: _ } => todo!(),
        RunInstructionTraceStep::Call { index: _ } => todo!(),
        RunInstructionTraceStep::GetLocal { depth, value } => {
            vec![
                MemoryEvent::new(
                    eid,
                    mmid,
                    depth.into(),
                    LocationType::Stack,
                    AccessType::Read,
                    // FIXME: use correct type
                    VarType::I32,
                    value.0,
                ),
                MemoryEvent::new(
                    eid,
                    mmid.into(),
                    0,
                    LocationType::Stack,
                    AccessType::Write,
                    // FIXME: use correct type
                    VarType::I32,
                    value.0,
                ),
            ]
        }
        RunInstructionTraceStep::I32Const { value } => mem_op_from_stack_only_step::<0, 1>(
            eid,
            mmid,
            VarType::I32,
            VarType::I32,
            &[],
            &[value as u64],
        ),
        RunInstructionTraceStep::I32BinOp { left, right, value } => {
            mem_op_from_stack_only_step::<2, 1>(
                eid,
                mmid,
                VarType::I32,
                VarType::I32,
                &[right as u64, left as u64],
                &[value as u64],
            )
        }
        RunInstructionTraceStep::I32Comp { left, right, value } => {
            mem_op_from_stack_only_step::<2, 1>(
                eid,
                mmid,
                VarType::I32,
                VarType::I32,
                &[right as u64, left as u64],
                &[value as u64],
            )
        }
    }
}
