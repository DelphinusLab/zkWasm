use crate::test::test_instruction;

#[test]
fn test_trivial_return() {
    let textual_repr = r#"
            (module
                (func (export "zkmain")
                  return
                )
               )
            "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_return_with_drop_ok() {
    let textual_repr = r#"
            (module
                (func (export "zkmain")
                  (block
                    (i32.const 0)
                    (i32.const 0)
                    return
                  )
                )
               )
            "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_return_with_keep_ok() {
    let textual_repr = r#"
            (module
                (func (export "zkmain") (result i32)
                    (i32.const 0)
                )
               )
            "#;

    test_instruction(textual_repr).unwrap();
}
