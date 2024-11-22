use crate::test::test_instruction;

#[test]
fn test_unary() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 0x00100000)
                (i32.ctz)
                (drop)

                (i32.const 0x00000001)
                (i32.ctz)
                (drop)

                (i32.const 0x80000000)
                (i32.ctz)
                (drop)

                (i32.const 0x00000000)
                (i32.ctz)
                (drop)

                (i64.const 0x0010000000000000)
                (i64.ctz)
                (drop)

                (i64.const 0x0000000000000001)
                (i64.ctz)
                (drop)

                (i64.const 0x8000000000000000)
                (i64.ctz)
                (drop)

                (i64.const 0x0000000000000000)
                (i64.ctz)
                (drop)

                (i32.const 0x00000001)
                (i32.clz)
                (drop)

                (i32.const 0x80000000)
                (i32.clz)
                (drop)

                (i32.const 0x00000000)
                (i32.clz)
                (drop)

                (i32.const 0xffffffff)
                (i32.clz)
                (drop)

                (i64.const 0x0000000000000001)
                (i64.clz)
                (drop)

                (i64.const 0x8000000000000000)
                (i64.clz)
                (drop)

                (i64.const 0x0000000000000000)
                (i64.clz)
                (drop)

                (i64.const 0xffffffffffffffff)
                (i64.clz)
                (drop)

                  (i32.const 0x00000000)
                  (i32.popcnt)
                  (drop)

                  (i32.const 0xffffffff)
                  (i32.popcnt)
                  (drop)

                  (i64.const 0x0000000000000000)
                  (i64.popcnt)
                  (drop)

                  (i64.const 0xffffffffffffffff)
                  (i64.popcnt)
                  (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}
