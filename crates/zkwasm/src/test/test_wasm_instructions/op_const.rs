use crate::test::test_instruction;

#[test]
fn test_const() {
    let textual_repr = r#"
            (module
                (func (export "zkmain")
                  (i32.const 0)
                  (drop)
                  (i64.const 0)
                  (drop)
                )
               )
            "#;

    test_instruction(textual_repr).unwrap();
}
