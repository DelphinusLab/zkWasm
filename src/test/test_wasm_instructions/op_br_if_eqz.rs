
use crate::test::test_circuit_noexternal;

#[test]
fn test_br_if_eqz_ok() {
    let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (if (i32.const 0)
                  (then (br 0))
                  (else (br 0))
                )
              )
            )
           )
        "#;

    test_circuit_noexternal(textual_repr).unwrap();
}
