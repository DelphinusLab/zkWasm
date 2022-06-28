use crate::{etable::Event, itable::Inst};
use wasmi::tracer::Tracer;

pub struct CircuitBuilder {
    pub(crate) itable: Vec<Inst>,
    pub(crate) etable: Vec<Event>,
}

impl CircuitBuilder {
    pub fn from_tracer(tracer: Tracer) -> CircuitBuilder {
        Self {
            itable: tracer
                .itable
                .0
                .into_iter()
                .map(|ientry| Inst::from(ientry))
                .collect(),
            etable: tracer
                .etable
                .0
                .into_iter()
                .map(|eentry| Event::from(eentry))
                .collect(),
        }
    }
}
