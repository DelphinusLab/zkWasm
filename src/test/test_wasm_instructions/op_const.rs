#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_const() {
        let textual_repr = r#"
            (module
                (func (export "test")
                  (i32.const 0)
                  (drop)
                  (i64.const 0)
                  (drop)
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
