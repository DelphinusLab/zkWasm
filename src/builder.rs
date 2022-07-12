use crate::runtime::memory_event_of_step;
use crate::spec::etable::EventTableEntry;
use crate::spec::itable::InstructionTableEntry;
use crate::spec::mtable::MemoryTableEntry;
use wasmi::tracer::Tracer;

pub(crate) const VAR_COLUMNS: usize = 50;

#[derive(Default, Clone)]
pub struct CircuitBuilder {
    pub(crate) itable: Vec<InstructionTableEntry>,
    pub(crate) etable: Vec<EventTableEntry>,
    pub(crate) mtable: Vec<MemoryTableEntry>,
}

impl CircuitBuilder {
    pub fn from_tracer(tracer: &Tracer) -> CircuitBuilder {
        let itable = tracer
            .itable
            .0
            .iter()
            .map(|ientry| InstructionTableEntry::from(ientry))
            .collect();

        let etable = tracer
            .etable
            .0
            .iter()
            .map(|eentry| EventTableEntry::from(eentry))
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

mod test {
    use halo2_proofs::arithmetic::FieldExt;

    use crate::test::test_circuit::TestCircuit;

    use super::*;

    impl CircuitBuilder {
        pub fn new_test_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
            TestCircuit::new(&self)
        }
    }
}
