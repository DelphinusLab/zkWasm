cfg_if::cfg_if! {
    if #[cfg(feature = "continuation")] {
        use crate::circuits::cell::AllocatedU32Cell;

        pub(crate) type AllocatedU32StateCell<F> = AllocatedU32Cell<F>;
    } else {
        use crate::circuits::cell::AllocatedCommonRangeCell;

        pub(crate) type AllocatedU32StateCell<F> = AllocatedCommonRangeCell<F>;
    }
}
