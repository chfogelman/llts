pub use core::hint::select_unpredictable;

/// Marks the branch on which this hint appears as cold.
///
/// Stable implementation of [`std::hint::cold_path`]. The compiler will pessimize this branch,
/// possibly at the benefit of alternative branches.
#[cold]
#[inline]
pub const fn cold_path() {}

/// Hint to the compiler that the result of a boolean expression is likely to be `true`
///
/// Stable implementation of [`std::hint::likely`]. The compiler will assume the boolean condition
/// is true and pessimize alternative branches, possibly at the benefit of likely branches.
/// However, it may also hurt performance of both the likely and unlikely branches, or of the
/// program as a whole, and thus should be used sparingly and only with benchmarks to prove
/// benefit.
#[inline]
#[must_use]
pub const fn likely(b: bool) -> bool {
    if !b {
        cold_path();
    }
    b
}

/// Hint to the compiler that the result of a boolean expression is *un*likely to be `true`
///
/// Stable implementation of [`std::hint::unlikely`]. The compiler will assume the boolean condition
/// is false and pessimize this branch, possibly at the benefit of likely branches.
/// However, it may also hurt performance of both the likely and unlikely branches, or of the
/// program as a whole, and thus should be used sparingly and only with benchmarks to prove
/// benefit.
#[inline]
#[must_use]
pub const fn unlikely(b: bool) -> bool {
    if b {
        cold_path();
    }
    b
}
