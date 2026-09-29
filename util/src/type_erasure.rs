use std::marker::PhantomData;
use std::mem::MaybeUninit;

/// A type-erased closure object of at most `N` bytes in size.
///
/// Unlike [`Box<dyn Fn(&mut Dst) -> R>`], this requires no dynamic allocation.
pub struct TypeErasedFn<const N: usize, Dst, R = ()> {
    buf:         [std::mem::MaybeUninit<u8>; N],
    dispatch_fn: unsafe fn(*const (), &mut Dst) -> R,
    _pd:         PhantomData<Box<dyn FnOnce(&mut Dst) -> R>>,
}

impl<const N: usize, Dst, R> TypeErasedFn<N, Dst, R> {
    /// Type-erase a closure object. This can then be called with [`TypeErasedFn::call`]
    ///
    /// Note that this function consumes the closure, but does not inherently drop it. The owned
    /// fields of the closure will _only_ be dropped when [`TypeErasedFn::call`] is invoked.
    ///
    /// Panics if the function is larger than the available buffer size.
    pub fn new<F: FnOnce(&mut Dst) -> R>(f: F) -> Self {
        assert!(
            std::mem::size_of::<F>() <= N,
            "Function {} is larger than {} bytes",
            std::any::type_name::<F>(),
            N
        );
        unsafe fn dispatcher<F: FnOnce(&mut Dst) -> R, Dst, R>(buf: *const (), dst: &mut Dst) -> R {
            let f = unsafe { std::ptr::read_unaligned(buf as *const F) };
            f(dst)
        }
        let buf = unsafe {
            // SAFETY: The assert at the top guarantees that `f` is no larger than the provided
            // buffer.
            let mut buf = [MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut F, f);
            buf
        };
        TypeErasedFn {
            buf,
            dispatch_fn: dispatcher::<F, Dst, R>,
            _pd: PhantomData,
        }
    }

    /// Call the type-erased closure object on its destination.
    ///
    /// This consumes the erased function object
    pub fn call(self, dst: &mut Dst) -> R {
        let Self {
            dispatch_fn,
            ref buf,
            _pd,
        } = self;
        unsafe {
            // SAFETY: We are assured here that we're calling the dispatch function on the correct buffer
            dispatch_fn(buf.as_ptr() as *const (), dst)
        }
    }
}

/// A type-erased closure object of at most `N` bytes in size.
///
/// Unlike [`Box<dyn Fn(&mut Dst) -> R>`], this requires no dynamic allocation.
pub struct TypeErasedSendFn<const N: usize, Dst, R = ()> {
    buf:         [std::mem::MaybeUninit<u8>; N],
    dispatch_fn: unsafe fn(*const (), &mut Dst) -> R,
    _pd:         PhantomData<Box<dyn FnOnce(&mut Dst) -> R + Send>>,
}

impl<const N: usize, Dst, R> TypeErasedSendFn<N, Dst, R> {
    /// Type-erase a closure object. This can then be called with [`TypeErasedFn::call`]
    ///
    /// Note that this function consumes the closure, but does not inherently drop it. The owned
    /// fields of the closure will _only_ be dropped when [`TypeErasedFn::call`] is invoked.
    ///
    /// Panics if the function is larger than the available buffer size.
    pub fn new<F: FnOnce(&mut Dst) -> R + Send>(f: F) -> Self {
        assert!(
            std::mem::size_of::<F>() <= N,
            "Function {} is larger than {} bytes",
            std::any::type_name::<F>(),
            N
        );
        unsafe fn dispatcher<F: FnOnce(&mut Dst) -> R, Dst, R>(buf: *const (), dst: &mut Dst) -> R {
            let f = unsafe { std::ptr::read_unaligned(buf as *const F) };
            f(dst)
        }
        let buf = unsafe {
            // SAFETY: The assert at the top guarantees that `f` is no larger than the provided
            // buffer.
            let mut buf = [MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut F, f);
            buf
        };
        TypeErasedSendFn {
            buf,
            dispatch_fn: dispatcher::<F, Dst, R>,
            _pd: PhantomData,
        }
    }

    /// Call the type-erased closure object on its destination.
    ///
    /// This consumes the erased function object
    pub fn call(self, dst: &mut Dst) -> R {
        let Self {
            dispatch_fn,
            ref buf,
            _pd,
        } = self;
        unsafe {
            // SAFETY: We are assured here that we're calling the dispatch function on the correct buffer
            dispatch_fn(buf.as_ptr() as *const (), dst)
        }
    }
}
#[cfg(test)]
mod test;
