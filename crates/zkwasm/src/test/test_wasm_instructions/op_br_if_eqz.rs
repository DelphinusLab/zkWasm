use crate::test::test_instruction;

#[test]
fn test_br_if_eqz_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (if (i32.const 0)
                  (then (br 0))
                  (else (br 0))
                )
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}
