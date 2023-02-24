#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_local_set() {
        let textual_repr = r#"
            (module
                (func (export "test")
                    (local i32 i64)
                    (i32.const 0)
                    (local.set 0)
                    (i64.const 0)
                    (local.set 1)
                )
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
