use crate::test::test_circuit_noexternal;

#[test]
fn test_memory_size() {
    let textual_repr = r#"
        (module
            (memory 2)

            (func (export "test")
                (memory.size)
                (drop)
            )
        )
    "#;

    test_circuit_noexternal(textual_repr).unwrap()
}
