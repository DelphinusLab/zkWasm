use crate::test::test_instruction;

#[test]
fn test_bin_shift_shl() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 12)
                (i32.const 0)
                (i32.shl)
                (drop)
                (i32.const 12)
                (i32.const 1)
                (i32.shl)
                (drop)
                (i32.const 12)
                (i32.const 33)
                (i32.shl)
                (drop)
                (i32.const 4294967295)
                (i32.const 1)
                (i32.shl)
                (drop)

                (i64.const 12)
                (i64.const 0)
                (i64.shl)
                (drop)
                (i64.const 12)
                (i64.const 1)
                (i64.shl)
                (drop)
                (i64.const 12)
                (i64.const 67)
                (i64.shl)
                (drop)
                (i64.const 0xffffffffffffffff)
                (i64.const 1)
                (i64.shl)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_bin_shift_shr_u() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 12)
                (i32.const 0)
                (i32.shr_u)
                (drop)
                (i32.const 12)
                (i32.const 3)
                (i32.shr_u)
                (drop)
                (i32.const 12)
                (i32.const 35)
                (i32.shr_u)
                (drop)

                (i64.const 12)
                (i64.const 3)
                (i64.shr_u)
                (drop)
                (i64.const 12)
                (i64.const 0)
                (i64.shr_u)
                (drop)
                (i64.const 12)
                (i64.const 68)
                (i64.shr_u)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_bin_shift_shr_s() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 12)
                (i32.const 0)
                (i32.shr_s)
                (drop)
                (i32.const 23)
                (i32.const 2)
                (i32.shr_s)
                (drop)
                (i32.const -23)
                (i32.const 5)
                (i32.shr_s)
                (drop)
                (i32.const 0)
                (i32.const 5)
                (i32.shr_s)
                (drop)
                (i32.const 23)
                (i32.const 35)
                (i32.shr_s)
                (drop)
                (i32.const -23)
                (i32.const 35)
                (i32.shr_s)
                (drop)
                (i32.const -1)
                (i32.const 5)
                (i32.shr_s)
                (drop)

                (i64.const 23)
                (i64.const 2)
                (i64.shr_s)
                (drop)
                (i64.const -23)
                (i64.const 5)
                (i64.shr_s)
                (drop)
                (i64.const 0)
                (i64.const 5)
                (i64.shr_s)
                (drop)
                (i64.const 12)
                (i64.const 0)
                (i64.shr_s)
                (drop)
                (i64.const 23)
                (i64.const 68)
                (i64.shr_s)
                (drop)
                (i64.const -23)
                (i64.const 68)
                (i64.shr_s)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_bin_shift_rotl() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 12)
                (i32.const 0)
                (i32.rotl)
                (drop)
                (i32.const 23)
                (i32.const 5)
                (i32.rotl)
                (drop)
                (i32.const 2863311530)
                (i32.const 5)
                (i32.rotl)
                (drop)

                (i64.const 12)
                (i64.const 0)
                (i64.rotl)
                (drop)
                (i64.const 23)
                (i64.const 5)
                (i64.rotl)
                (drop)
                (i64.const 2863311530)
                (i64.const 5)
                (i64.rotl)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_bin_shift_rotr() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
                (i32.const 12)
                (i32.const 0)
                (i32.rotr)
                (drop)
                (i32.const 23)
                (i32.const 5)
                (i32.rotr)
                (drop)

                (i64.const 12)
                (i64.const 0)
                (i64.rotr)
                (drop)
                (i64.const 23)
                (i64.const 5)
                (i64.rotr)
                (drop)
            )
        )
        "#;

    test_instruction(textual_repr).unwrap()
}
