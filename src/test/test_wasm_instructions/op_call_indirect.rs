use crate::test::test_circuit_noexternal;

#[test]
fn test_call_indirect() {
    let textual_repr = r#"
        (module
            (type (;0;) (func (param i32 i32) (result i32)))
            (func (;0;) (type 0) (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add)
            (func (;1;) (type 0) (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.sub)
            (func (;2;) (result i32)
                i32.const 1
                i32.const 2
                i32.const 1
                call_indirect (type 0))
            (table (;0;) 2 2 funcref)
            (export "test" (func 2))
            (elem (;0;) (i32.const 0) func 0 1)
        )
    "#;

    test_circuit_noexternal(textual_repr).unwrap()
}