use crate::test::test_circuit_noexternal;

#[test]
fn test_load_normal() {
    let textual_repr = r#"
        (module
            (memory $0 1)
            (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
            (func (export "test")
                (i32.const 0)
                (i64.load offset=0)
                (drop)
                (i32.const 4)
                (i64.load offset=4)
                (drop)
                (i32.const 0)
                (i64.load32_u offset=0)
                (drop)
                (i32.const 0)
                (i64.load32_s offset=0)
                (drop)
                (i32.const 0)
                (i64.load16_u offset=0)
                (drop)
                (i32.const 0)
                (i64.load16_s offset=0)
                (drop)
                (i32.const 0)
                (i64.load8_u offset=0)
                (drop)
                (i32.const 0)
                (i64.load8_s offset=0)
                (drop)

                (i32.const 0)
                (i32.load offset=0)
                (drop)
                (i32.const 4)
                (i32.load offset=0)
                (drop)
                (i32.const 0)
                (i32.load16_u offset=0)
                (drop)
                (i32.const 0)
                (i32.load16_s offset=0)
                (drop)
                (i32.const 0)
                (i32.load8_u offset=0)
                (drop)
                (i32.const 0)
                (i32.load8_s offset=0)
                (drop)
            )
        )
    "#;

    test_circuit_noexternal(textual_repr).unwrap();
}

#[test]
fn test_load_cross() {
    let textual_repr = r#"
            (module
                (memory $0 1)
                (data (i32.const 0) "\ff\00\00\00\fe\00\00\00\fd\00\00\00\fc\00\00\00")
                (func (export "test")
                    (i32.const 4)
                    (i64.load offset=0)
                    (drop)
                    (i32.const 6)
                    (i64.load32_u offset=0)
                    (drop)
                    (i32.const 7)
                    (i64.load16_u offset=0)
                    (drop)

                    (i32.const 6)
                    (i32.load offset=0)
                    (drop)
                    (i32.const 7)
                    (i32.load16_u offset=0)
                    (drop)
                )
               )
            "#;

    test_circuit_noexternal(textual_repr).unwrap();
}

#[test]
fn test_load_memory_overflow_circuit() {
    let textual_repr = r#"
        (module
            (memory $0 26)
            (func (export "test")
                (i32.const 0)
                (i64.load offset=1638400)
                (drop)
            )
        )
    "#;

    assert!(test_circuit_noexternal(textual_repr).is_err());
}

#[test]
fn test_load_maximal_memory() {
    // k18 support 10 pages at most.
    let textual_repr = r#"
        (module
            (memory $0 10)
            (func (export "test")
                (i32.const 0)
                (i64.load offset=655352)
                (drop)

                (i32.const 655352)
                (i64.load offset=0)
                (drop)

                (i32.const 0)
                (i64.load offset=0)
                (drop)
            )
        )
    "#;

    test_circuit_noexternal(textual_repr).unwrap();
}
