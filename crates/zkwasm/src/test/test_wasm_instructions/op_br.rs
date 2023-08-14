use crate::test::test_circuit_noexternal;

#[test]
fn test_br_ok() {
    let textual_repr = r#"
        (module
            (func (export "test")
              (block
                br 0
              )
            )
           )
        "#;

    test_circuit_noexternal(textual_repr).unwrap();
}

#[test]
fn test_br_drop_ok() {
    let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 0)
                br 0
              )
            )
           )
        "#;

    test_circuit_noexternal(textual_repr).unwrap();
}
