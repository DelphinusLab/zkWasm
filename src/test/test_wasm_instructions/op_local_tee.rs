use crate::test::test_circuit_noexternal;

#[test]
fn test_local_tee() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (local i32 i64)
                (i32.const 0)
                (local.tee 0)
                (drop)
                (i64.const 0)
                (local.tee 1)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}
