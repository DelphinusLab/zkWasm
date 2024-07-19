use crate::test::test_instruction;

#[test]
fn test_eqz() {
    let textual_repr = r#"
                (module
                    (func (export "zkmain")
                      (i32.const 0)
                      (i32.eqz)
                      (drop)

                      (i32.const 1)
                      (i32.eqz)
                      (drop)

                      (i64.const 0)
                      (i64.eqz)
                      (drop)

                      (i64.const 1)
                      (i64.eqz)
                      (drop)
                    )
                   )
                "#;

    test_instruction(textual_repr).unwrap()
}
