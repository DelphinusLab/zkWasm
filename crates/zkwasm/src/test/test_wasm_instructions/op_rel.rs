use std::vec;

use crate::test::test_instruction;

#[test]
fn test_op_rel() {
    let tys = vec![
        ("i32", vec!["0", "1", "2", "-1", "-2", "0x80000000"]),
        (
            "i64",
            vec![
                "0",
                "1",
                "2",
                "-1",
                "-2",
                "-0x100000001",
                "-0x100000002",
                "0x100000001",
                "0x100000002",
                "0x8000000000000000",
            ],
        ),
    ];
    let ops = vec![
        "gt_u", "ge_u", "lt_u", "le_u", "eq", "ne", "gt_s", "ge_s", "lt_s", "le_s",
    ];

    let mut textual_repr = r#"
            (module
                (func (export "zkmain")"#
        .to_owned();

    for (t, values) in tys {
        for op in ops.iter() {
            for l in values.iter() {
                for r in values.iter() {
                    textual_repr = format!(
                        r#"{}
                            ({}.const {})
                            ({}.const {})
                            ({}.{})
                            (drop)
                            "#,
                        textual_repr, t, l, t, r, t, op
                    );
                }
            }
        }
    }

    textual_repr = format!("{}))", textual_repr);
    test_instruction(&textual_repr).unwrap()
}
