use crate::test::test_instruction;

#[test]
fn test_select() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (i32.const 1)
              (i32.const 2)
              (i32.const 0)
              select
              drop
              (i64.const 1)
              (i64.const 2)
              (i32.const 1)
              select
              drop
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}
