use crate::test::test_circuit_noexternal;

#[test]
fn test_store_normal() {
    let textual_repr = r#"
        (module
            (memory $0 1)
            (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
            (func (export "test")
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

    test_circuit_noexternal(textual_repr).unwrap();
}
