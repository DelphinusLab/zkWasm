use crate::test::test_circuit_noexternal;

#[test]
fn test_select() {
    let textual_repr = r#"
        (module
            (func (export "test")
              (i32.const 1)
              (i32.const 2)
              (i32.const 0)
              select
              drop
              (i64.const 1)
              (i64.const 2)
              (i32.const 0)
              select
              drop
            )
           )
        "#;

    test_circuit_noexternal(textual_repr).unwrap();
}
