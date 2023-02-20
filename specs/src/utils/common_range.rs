use num_bigint::BigUint;
use serde::Serialize;
use std::ops::{Add, AddAssign, Deref, Sub, SubAssign};

pub const COMMON_RANGE_OFFSET: u32 = 32;

/// A type with varying lengths depending on the size of the circuit.
/// Ranges from '0' to '1 << (k - 1)'.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CommonRange {
    internal: u32,
}

impl Deref for CommonRange {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl Add<u32> for CommonRange {
    type Output = CommonRange;

    fn add(self, rhs: u32) -> Self::Output {
        CommonRange {
            internal: self.internal + rhs,
        }
    }
}

impl Add for CommonRange {
    type Output = CommonRange;

    fn add(self, rhs: Self) -> Self::Output {
        CommonRange {
            internal: self.internal + rhs.internal,
        }
    }
}

impl AddAssign<u32> for CommonRange {
    fn add_assign(&mut self, rhs: u32) {
        self.internal += rhs
    }
}

impl AddAssign for CommonRange {
    fn add_assign(&mut self, rhs: Self) {
        self.internal += rhs.internal
    }
}

impl Sub<u32> for CommonRange {
    type Output = CommonRange;

    fn sub(self, rhs: u32) -> Self::Output {
        CommonRange {
            internal: self.internal - rhs,
        }
    }
}

impl Sub for CommonRange {
    type Output = CommonRange;

    fn sub(self, rhs: Self) -> Self::Output {
        CommonRange {
            internal: self.internal - rhs.internal,
        }
    }
}

impl SubAssign for CommonRange {
    fn sub_assign(&mut self, rhs: Self) {
        self.internal -= rhs.internal
    }
}

impl SubAssign<u32> for CommonRange {
    fn sub_assign(&mut self, rhs: u32) {
        self.internal -= rhs
    }
}

impl From<CommonRange> for u32 {
    fn from(value: CommonRange) -> Self {
        value.internal
    }
}

impl From<u32> for CommonRange {
    fn from(value: u32) -> Self {
        Self { internal: value }
    }
}

impl From<CommonRange> for BigUint {
    fn from(value: CommonRange) -> Self {
        BigUint::from(value.internal)
    }
}

impl CommonRange {
    pub fn checked_add(self, v: u32) -> Option<CommonRange> {
        self.internal.checked_add(v).map(|v| CommonRange::from(v))
    }
}
