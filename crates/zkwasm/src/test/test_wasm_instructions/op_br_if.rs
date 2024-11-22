use crate::test::test_instruction;

#[test]
fn test_br_if_trivial_nojump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (i32.const 0)
                br_if 0
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_if_trivial_jump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (i32.const 1)
                br_if 0
                (i32.const 0)
                drop
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_if_block_with_arg_do_not_jump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block (result i32)
                (i32.const 0)
                (i32.const 0)
                br_if 0
              )
              drop
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_if_block_with_arg_do_jump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block (result i32)
                (i32.const 0)
                (i32.const 1)
                br_if 0
              )
              drop
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_if_block_with_drop_do_not_jump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 0)
                  br_if 1
                  drop
                  drop
                )
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}

#[test]
fn test_br_if_block_with_drop_do_jump_ok() {
    let textual_repr = r#"
        (module
            (func (export "zkmain")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 1)
                  br_if 1
                  drop
                  drop
                )
              )
            )
           )
        "#;

    test_instruction(textual_repr).unwrap();
}
