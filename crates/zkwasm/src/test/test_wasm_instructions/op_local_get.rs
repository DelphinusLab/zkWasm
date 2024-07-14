use crate::test::test_instruction;

#[test]
fn test_local_get() {
    let textual_repr = r#"
            (module
                (func (export "zkmain")
                    (local i32 i64)
                    (local.get 0)
                    (drop)
                    (local.get 1)
                    (drop)
                )
            )
            "#;

    test_instruction(textual_repr).unwrap()
}
