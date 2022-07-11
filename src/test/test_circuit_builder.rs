use std::{cell::RefCell, rc::Rc};

use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, plonk::Error};
use wasmi::{ModuleRef, NopExternals};

use crate::builder::CircuitBuilder;

const K: u32 = 5;

pub fn run_test_circuit<F: FieldExt>(instance: &ModuleRef) -> Result<(), Error> {
    let mut tracer = wasmi::tracer::Tracer::default();
    tracer.register_module_instance(instance);
    let tracer = Rc::new(RefCell::new(tracer));

    // Finally, invoke exported function "test" with no parameters
    // and empty external function executor.
    assert_eq!(
        instance
            .invoke_export("test", &[], &mut NopExternals,)
            .expect("failed to execute export"),
        None,
    );

    let builder = CircuitBuilder::from_tracer(&tracer.borrow());
    let circuit = builder.new_test_circuit::<F>();

    MockProver::run(K, &circuit, vec![])?;

    Ok(())
}
