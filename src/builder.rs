use crate::{etable::Event, itable::Inst, mtable::MemoryEvent, opcode::memory_event_of_step};
use wasmi::tracer::Tracer;

pub struct CircuitBuilder {
    pub(crate) itable: Vec<Inst>,
    pub(crate) etable: Vec<Event>,
    pub(crate) mtable: Vec<MemoryEvent>,
}

impl CircuitBuilder {
    pub fn from_tracer(tracer: Tracer) -> CircuitBuilder {
        let itable = tracer
            .itable
            .0
            .into_iter()
            .map(|ientry| Inst::from(ientry))
            .collect();

        let etable = tracer
            .etable
            .0
            .into_iter()
            .map(|eentry| Event::from(eentry))
            .collect::<Vec<_>>();

        let mtable = etable
            .iter()
            .map(|eentry| memory_event_of_step(eentry))
            .collect::<Vec<Vec<_>>>();
        // concat vectors without Clone
        let mtable = mtable.into_iter().flat_map(|x| x.into_iter()).collect();

        Self {
            itable,
            etable,
            mtable,
        }
    }
}
