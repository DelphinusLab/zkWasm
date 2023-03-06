#[macro_export]
macro_rules! exec_with_profile {
    ($desc:expr, $statement: expr) => {{
        let timer = start_timer!($desc);
        let r = $statement;
        end_timer!(timer);
        r
    }};
}
