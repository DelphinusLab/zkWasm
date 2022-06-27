use crate::itable::Inst;
use wasmi::tracer::Tracer;

pub struct CircuitBuilder {
    pub(crate) itable: Vec<Inst>,
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
        }
    }
}
