pub mod align;
pub mod arith;
pub mod bstr;
pub mod fixed_point;
pub mod hint;
pub mod lockfree;
pub mod logger;
pub mod queue;
pub mod telemetry;
pub mod type_erasure;
pub mod unmut;

pub use bstr::FixedString;
pub use fixed_point::*;

#[macro_export]
macro_rules! const_assert_sizeof {
    ($t:ty, $op:tt, $size:expr $(,$msg:tt)*) => {
        struct __Check<T>(::std::marker::PhantomData<T>);
        impl<T> __Check<T> {
            const VALID: () = ::std::assert!(::std::mem::size_of::<T>() $op $size, $($msg)*);
        }
        #[allow(clippy::let_unit_value)]
        let _ = __Check::<$t>::VALID;
    }
}

#[macro_export]
macro_rules! const_assert_generic {
    ($lo:expr, $op:tt, $hi:expr $(,$msg:tt)*) => {
        struct __Check<const N: usize, const M: usize>;
        impl<const N: usize, const M: usize> __Check<N, M> {
            const VALID: () = ::std::assert!(N $op M, $($msg)*);
        }
        #[allow(clippy::let_unit_value)]
        let _ = __Check::<$lo, $hi>::VALID;
    }
}
