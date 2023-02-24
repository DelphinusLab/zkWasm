use crate::test::test_circuit_noexternal;

#[test]
fn test_bin_add() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 1)
                (i32.const 1)
                (i32.add)
                (drop)
                (i32.const 1)
                (i32.const 4294967295)
                (i32.add)
                (drop)

                (i64.const 1)
                (i64.const 1)
                (i64.add)
                (drop)
                (i64.const 1)
                (i64.const 18446744073709551615)
                (i64.add)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_sub() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 1)
                (i32.const 1)
                (i32.sub)
                (drop)
                (i32.const 0)
                (i32.const 1)
                (i32.sub)
                (drop)

                (i64.const 1)
                (i64.const 1)
                (i64.sub)
                (drop)
                (i64.const 0)
                (i64.const 1)
                (i64.sub)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_mul() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 4)
                (i32.const 3)
                (i32.mul)
                (drop)
                (i32.const 4294967295)
                (i32.const 4294967295)
                (i32.mul)
                (drop)

                (i64.const 4)
                (i64.const 3)
                (i64.mul)
                (drop)
                (i64.const 18446744073709551615)
                (i64.const 18446744073709551615)
                (i64.mul)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_div_u() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 4)
                (i32.const 3)
                (i32.div_u)
                (drop)
                (i32.const 4)
                (i32.const 4)
                (i32.div_u)
                (drop)
                (i32.const 0x80000000)
                (i32.const 1)
                (i32.div_u)
                (drop)

                (i64.const 4)
                (i64.const 3)
                (i64.div_u)
                (drop)
                (i64.const 4)
                (i64.const 4)
                (i64.div_u)
                (drop)
                (i64.const 0x8000000000000000)
                (i64.const 1)
                (i64.div_u)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_div_s() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 4)
                (i32.const 3)
                (i32.div_s)
                (drop)
                (i32.const -4)
                (i32.const -3)
                (i32.div_s)
                (drop)
                (i32.const -4)
                (i32.const 3)
                (i32.div_s)
                (drop)
                (i32.const 4)
                (i32.const -3)
                (i32.div_s)
                (drop)
                (i32.const -3)
                (i32.const 4)
                (i32.div_s)
                (drop)
                (i32.const 0x80000000)
                (i32.const 1)
                (i32.div_s)
                (drop)

                (i64.const 4)
                (i64.const 3)
                (i64.div_s)
                (drop)
                (i64.const -4)
                (i64.const -3)
                (i64.div_s)
                (drop)
                (i64.const -4)
                (i64.const 3)
                (i64.div_s)
                (drop)
                (i64.const 4)
                (i64.const -3)
                (i64.div_s)
                (drop)
                (i64.const -3)
                (i64.const 4)
                (i64.div_s)
                (drop)
                (i64.const 0x8000000000000000)
                (i64.const 1)
                (i64.div_s)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_rem_u() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 4)
                (i32.const 3)
                (i32.rem_u)
                (drop)
                (i32.const 4)
                (i32.const 4)
                (i32.rem_u)
                (drop)

                (i64.const 4)
                (i64.const 3)
                (i64.rem_u)
                (drop)
                (i64.const 4)
                (i64.const 4)
                (i64.rem_u)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}

#[test]
fn test_bin_rem_s() {
    let textual_repr = r#"
        (module
            (func (export "test")
                (i32.const 4)
                (i32.const 3)
                (i32.rem_s)
                (drop)
                (i32.const -4)
                (i32.const -3)
                (i32.rem_s)
                (drop)
                (i32.const -4)
                (i32.const 3)
                (i32.rem_s)
                (drop)
                (i32.const 4)
                (i32.const -3)
                (i32.rem_s)
                (drop)
                (i32.const 4)
                (i32.const -4)
                (i32.rem_s)
                (drop)

                (i64.const 4)
                (i64.const 3)
                (i64.rem_s)
                (drop)
                (i64.const -4)
                (i64.const -3)
                (i64.rem_s)
                (drop)
                (i64.const -4)
                (i64.const 3)
                (i64.rem_s)
                (drop)
                (i64.const 4)
                (i64.const -3)
                (i64.rem_s)
                (drop)
                (i64.const 4)
                (i64.const -4)
                (i64.rem_s)
                (drop)
            )
        )
        "#;

    test_circuit_noexternal(textual_repr).unwrap()
}
