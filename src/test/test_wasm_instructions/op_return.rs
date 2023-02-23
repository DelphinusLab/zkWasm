#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_trivial_return() {
        let textual_repr = r#"
            (module
                (func (export "test")
                  return
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_return_with_drop_ok() {
        let textual_repr = r#"
            (module
                (func (export "test")
                  (block
                    (i32.const 0)
                    (i32.const 0)
                    return
                  )
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_return_with_keep_ok() {
        let textual_repr = r#"
            (module
                (func (export "test") (result i32)
                    (i32.const 0)
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
