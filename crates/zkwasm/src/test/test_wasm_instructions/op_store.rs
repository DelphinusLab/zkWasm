use crate::test::test_instruction;

#[test]
fn test_store_normal() {
    let textual_repr = r#"
        (module
            (memory $0 1)
            (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
            (func (export "zkmain")
                (i32.const 0)
                (i64.const 0)
                (i64.store offset=0)
                (i32.const 0)
                (i32.const 0)
                (i32.store offset=4)
                (i32.const 0)
                (i64.const 0x432134214)
                (i64.store offset=0)
                (i32.const 0)
                (i64.const 0)
                (i64.store32 offset=0)
                (i32.const 0)
                (i64.const 0)
                (i64.store16 offset=0)
                (i32.const 0)
                (i64.const 0)
                (i64.store8 offset=0)

                (i32.const 0)
                (i32.const 0)
                (i32.store offset=0)
                (i32.const 4)
                (i32.const 0)
                (i32.store offset=0)
                (i32.const 0)
                (i32.const 0)
                (i32.store16 offset=0)
                (i32.const 0)
                (i32.const 0)
                (i32.store8 offset=0)
                (i32.const 0)
                (i32.const 256)
                (i32.store8 offset=0)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_store_cross() {
    let textual_repr = r#"
            (module
                (memory $0 1)
                (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                (func (export "zkmain")
                    (i32.const 6)
                    (i64.const 32)
                    (i64.store32 offset=0)

                    (i32.const 4)
                    (i64.const 64)
                    (i64.store offset=0)

                    (i32.const 7)
                    (i64.const 16)
                    (i64.store16 offset=0)

                    (i32.const 6)
                    (i32.const 32)
                    (i32.store offset=0)

                    (i32.const 7)
                    (i32.const 16)
                    (i32.store16 offset=0)
                )
               )
            "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_store_large_memory() {
    let textual_repr = r#"
        (module
            (memory $0 10)
            (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
            (func (export "zkmain")
                (i32.const 0)
                (i32.const 16)
                (i32.store16 offset=655352)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}
