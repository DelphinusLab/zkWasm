mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_start_mock() {
        let textual_repr = r#"
        (module
            (func (;0;)
              i32.const 0
              drop
            )

            (func (;1;)
              i32.const 1
              drop
            )

            (start 0)
            (export "test" (func 1))
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
