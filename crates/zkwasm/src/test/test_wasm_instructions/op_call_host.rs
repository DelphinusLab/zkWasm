use specs::external_host_call_table::ExternalHostCallSignature;
use std::rc::Rc;
use wasmi::tracer::Observer;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::ForeignStatics;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use crate::test::test_circuit_with_env;

#[derive(Default)]
struct Context {
    acc: u64,
}
impl ForeignContext for Context {
    fn get_statics(&self) -> Option<ForeignStatics> {
        None
    }
}

#[test]
fn test_call_host_external() {
    let textual_repr = r#"
        (module
            (type (;0;) (func (result i64)))
            (type (;1;) (func (param i64)))
            (import "env" "foreign_push" (func (;0;) (type 1)))
            (import "env" "foreign_pop" (func (;1;) (type 0)))
            (func (;2;) (type 0) (result i64)
              i64.const 5
              call 0
              i64.const 10
              call 0
              i64.const 3
              call 0
              call 1)
            (memory (;0;) 1)
            (export "memory" (memory 0))
            (export "test" (func 2)))
        "#;

    let env = {
        let mut env = HostEnv::new();

        let foreign_playground_plugin = env
            .external_env
            .register_plugin("foreign_playground", Box::new(Context::default()));
        env.external_env.register_function(
            "foreign_push",
            0,
            ExternalHostCallSignature::Argument,
            foreign_playground_plugin.clone(),
            Rc::new(
                |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                    let context = context.downcast_mut::<Context>().unwrap();

                    let value: u64 = args.nth(0);
                    context.acc += value;

                    None
                },
            ),
        );
        env.external_env.register_function(
            "foreign_pop",
            1,
            ExternalHostCallSignature::Return,
            foreign_playground_plugin,
            Rc::new(
                |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                    let context = context.downcast_mut::<Context>().unwrap();

                    Some(wasmi::RuntimeValue::I64(context.acc as i64))
                },
            ),
        );

        env.finalize();

        env
    };

    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");
    test_circuit_with_env(env, WasmRuntimeIO::empty(), wasm, "test").unwrap();
}
