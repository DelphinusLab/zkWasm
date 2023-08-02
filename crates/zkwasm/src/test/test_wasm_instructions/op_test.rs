use crate::test::test_circuit_noexternal;

#[test]
fn test_eqz() {
    let textual_repr = r#"
                (module
                    (func (export "test")
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

    test_circuit_noexternal(textual_repr).unwrap()
}
