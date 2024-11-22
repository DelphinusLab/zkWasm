use crate::test::test_instruction;

#[test]
fn test_bin_bit() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 1)
                (i32.const 1)
                (i32.xor)
                (drop)
                (i32.const 21)
                (i32.const 31)
                (i32.xor)
                (drop)
                (i64.const 1)
                (i64.const 1)
                (i64.xor)
                (drop)
                (i64.const 21)
                (i64.const 31)
                (i64.xor)
                (drop)

                (i32.const 21)
                (i32.const 31)
                (i32.and)
                (drop)
                (i64.const 21)
                (i64.const 31)
                (i64.and)
                (drop)

                (i32.const 21)
                (i32.const 31)
                (i32.or)
                (drop)
                (i64.const 21)
                (i64.const 31)
                (i64.or)
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}
