use crate::test::test_instruction;

#[test]
fn test_call() {
    let textual_repr = r#"
        (module
            (func $foo (param i32) (result i32)
            (local i64 i32)
              i32.const 0
            )
            (func (export "zkmain")
              (i32.const 0)
              call $foo
              drop
            )
           )
        "#;

    test_instruction(textual_repr).unwrap()
}
