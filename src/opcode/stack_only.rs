use crate::mtable::{AccessType, LocationType, MemoryEvent, VarType};

pub(crate) fn mem_op_from_stack_only_step(
    eid: u64,
    mmid: u64,
    inputs_type: VarType,
    outputs_type: VarType,
    pop_value: &[u64],
    push_value: &[u64],
) -> Vec<MemoryEvent> {
    let mut mem_op = vec![];

    for i in 0..pop_value.len() {
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

    for i in 0..push_value.len() {
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
