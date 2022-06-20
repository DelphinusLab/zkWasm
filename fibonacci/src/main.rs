extern crate wabt;
extern crate wasmi;

use wasmi::{ImportsBuilder, ModuleInstance, NopExternals, RuntimeValue};

fn main() {
    // Parse WAT (WebAssembly Text format) into wasm bytecode.
    let wasm_binary: Vec<u8> = wabt::wat2wasm(
        r#"
            (module
                (memory $0 1)
                (export "memory" (memory $0))
                (export "fibonacci" (func $fibonacci))
                (func $fibonacci (; 0 ;) (param $0 i32) (result i32)
                 (block $label$0
                  (br_if $label$0
                   (i32.ne
                    (i32.or
                     (local.get $0)
                     (i32.const 1)
                    )
                    (i32.const 1)
                   )
                  )
                  (return
                   (local.get $0)
                  )
                 )
                 (i32.add
                  (call $fibonacci
                   (i32.add
                    (local.get $0)
                    (i32.const -1)
                   )
                  )
                  (call $fibonacci
                   (i32.add
                    (local.get $0)
                    (i32.const -2)
                   )
                  )
                 )
                )
               )
            "#,
    )
    .expect("failed to parse wat");

    let mut tracer = wasmi::tracer::Tracer::default();

    // Load wasm binary and prepare it for instantiation.
    let module = wasmi::Module::from_buffer(&wasm_binary).expect("failed to load wasm");

    // Instantiate a module with empty imports and
    // asserting that there is no `start` function.
    let instance = ModuleInstance::new(&module, &ImportsBuilder::default())
        .expect("failed to instantiate wasm module")
        .assert_no_start();

    tracer.register_module_instance(&instance);

    // Finally, invoke exported function "test" with no parameters
    // and empty external function executor.
    assert_eq!(
        instance
            .invoke_export_trace(
                "fibonacci",
                &[wasmi::RuntimeValue::I32(6)],
                &mut NopExternals,
                tracer,
            )
            .expect("failed to execute export"),
        Some(RuntimeValue::I32(8)),
    );

    //    println!("{:?}", tracer);
}
