use crate::test::test_circuit_noexternal;

#[test]
fn test_local_get() {
    let textual_repr = r#"
            (module
                (func (export "test")
                    (local i32 i64)
                    (local.get 0)
                    (drop)
                    (local.get 1)
                    (drop)
                
                )
            )
            "#;

    test_circuit_noexternal(textual_repr).unwrap()
}
