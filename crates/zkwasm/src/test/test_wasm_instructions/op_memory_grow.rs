use crate::test::test_instruction;

#[test]
fn test_memory_grow() {
    let textual_repr = r#"
        (module
            (memory 1 2)

            (func (export "zkmain")
                (memory.grow (i32.const 1))
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_memory_grow_fail() {
    let textual_repr = r#"
        (module
            (memory 1 2)

            (func (export "zkmain")
                (memory.grow (i32.const 2))
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_memory_grow_lazy_init() {
    let textual_repr = r#"
        (module
            (memory 0 1)

            (func (export "zkmain")
                (memory.grow (i32.const 1))
                (drop)
                (i32.const 0)
                (i32.load offset=0)
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}

#[test]
fn test_memory_grow_lazy_init_2() {
    let textual_repr = r#"
        (module
            (memory 1 2)

            (func (export "zkmain")
                (memory.grow (i32.const 1))
                (drop)
                (i32.const 65536)
                (i32.load offset=0)
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}
