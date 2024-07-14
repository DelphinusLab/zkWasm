use crate::test::test_instruction;

#[test]
fn test_br_table_1() {
    let textual_repr = r#"
        (module
            (func (export "zkmain") (result i32)
            (block
                (block
                (block
                    (block
                    (block
                        (br_table 3 2 1 0 4 (i32.const 0))
                        (return (i32.const 99))
                    )
                    (return (i32.const 100))
                    )
                    (return (i32.const 101))
                )
                (return (i32.const 102))
                )
            (return (i32.const 103))
            )
            (i32.const 104)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_table_2() {
    let textual_repr = r#"
        (module
            (func (export "zkmain") (result i32)
            (block
                (block
                (block
                    (block
                    (block
                        (br_table 3 2 1 0 4 (i32.const 4))
                        (return (i32.const 99))
                    )
                    (return (i32.const 100))
                    )
                    (return (i32.const 101))
                )
                (return (i32.const 102))
                )
                (return (i32.const 103))
            )
            (i32.const 104)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_table_oob_1() {
    let textual_repr = r#"
        (module
            (func (export "zkmain") (result i32)
            (block
                (block
                (block
                    (block
                    (block
                        (br_table 3 2 1 0 4 (i32.const 5))
                        (return (i32.const 99))
                    )
                    (return (i32.const 100))
                    )
                    (return (i32.const 101))
                )
                (return (i32.const 102))
                )
            (return (i32.const 103))
            )
            (i32.const 104)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_table_oob_2() {
    let textual_repr = r#"
        (module
            (func (export "zkmain") (result i32)
            (block
                (block
                (block
                    (block
                    (block
                        (br_table 3 2 1 0 4 (i32.const 99))
                        (return (i32.const 99))
                    )
                    (return (i32.const 100))
                    )
                    (return (i32.const 101))
                )
                (return (i32.const 102))
                )
            (return (i32.const 103))
            )
            (i32.const 104)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap();
}
