
use crate::test::test_circuit_noexternal;

#[test]
fn test_call() {
    let textual_repr = r#"
        (module
            (func $foo (param i32) (result i32)
            (local i64 i32)
              i32.const 0
            )
            (func (export "test")
              (i32.const 0)
              call $foo
              drop
            )
           )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}
