use crate::test::test_instruction;

#[test]
fn test_global_get() {
    let textual_repr = r#"
        (module
            (global $global_i32 i32 (i32.const 0))
            (global $global_i64 i64 (i64.const 0))

            (func (export "zkmain")
                (global.get $global_i32)
                (drop)
                (global.get $global_i64)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}
