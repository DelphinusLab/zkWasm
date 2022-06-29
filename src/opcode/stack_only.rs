use crate::mtable::{AccessType, LocationType, MemoryEvent, VarType};

pub(crate) fn mem_op_from_stack_only_step<const POP_SIZE: usize, const PUSH_SIZE: usize>(
    eid: u64,
    mmid: u64,
    inputs_type: VarType,
    outputs_type: VarType,
    pop_value: &[u64; POP_SIZE],
    push_value: &[u64; PUSH_SIZE],
) -> Vec<MemoryEvent> {
    let mut mem_op = vec![];

    for i in 0..POP_SIZE {
        mem_op.push(MemoryEvent::new(
            eid,
            mmid,
            i as u64,
            LocationType::Stack,
            AccessType::Read,
            inputs_type,
            pop_value[i],
        ));
    }

    for i in 0..PUSH_SIZE {
        mem_op.push(MemoryEvent::new(
            eid,
            mmid,
            i as u64,
            LocationType::Stack,
            AccessType::Write,
            outputs_type,
            push_value[i],
        ));
    }

    mem_op
}
