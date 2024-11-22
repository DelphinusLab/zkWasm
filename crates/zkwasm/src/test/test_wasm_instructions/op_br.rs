use crate::test::test_instruction;

#[test]
fn test_br_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                br 0
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_drop_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (i32.const 0)
                br 0
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}
