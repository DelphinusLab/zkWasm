use crate::test::test_instruction;

#[test]
fn test_memory_size() {
    let textual_repr = r#"
        (module
            (memory 2)

            (func (export "zkmain")
                (memory.size)
                (drop)
            )
        )
    "#;

    test_instruction(textual_repr).unwrap()
}
