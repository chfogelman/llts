use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::ops::Drop;
use std::ptr::NonNull;

/// The basic building block of an unmut collection
pub(super) struct Buffer<T> {
    buf: Cell<Option<NonNull<UnsafeCell<MaybeUninit<T>>>>>,
    cap: Cell<usize>,
}

impl<T> Buffer<T> {
    pub const fn new() -> Self {
        Self {
            buf: Cell::new(None),
            cap: Cell::new(0),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (buf, cap) = if capacity == 0 {
            (None, 0)
        } else {
            let mem = unsafe {
                // SAFETY: a [MaybeUninit<UnsafeCell<MaybeUninit<T>>>] is as initialized as it
                // needs to be.
                Box::leak(Box::new_uninit_slice(capacity).assume_init())
            };
            (NonNull::new(mem.as_mut_ptr()), mem.len())
        };
        Self {
            buf: Cell::new(buf),
            cap: Cell::new(cap),
        }
    }

    pub fn capacity(&self) -> usize {
        self.cap.get()
    }

    pub fn swap(&self, other: &Self) {
        self.buf.swap(&other.buf);
        self.cap.swap(&other.cap);
    }

    pub fn as_mut_ptr(&self) -> *mut UnsafeCell<MaybeUninit<T>> {
        self.buf.get().map_or(std::ptr::null_mut(), NonNull::as_ptr)
    }

    /// Get a pointer to the element at `offset` from the start of the buffer
    ///
    /// Preconditions: the buffer must be allocated and smaller than the offset
    pub unsafe fn get_unchecked_raw(&self, idx: usize) -> *mut MaybeUninit<T> {
        debug_assert!(self.buf.get().is_some());
        debug_assert!(idx < self.capacity());
        unsafe { self.buf.get().unwrap_unchecked().add(idx).as_ref().get() }
    }

    /// Get a mutable reference to the _initialized_ element at `offset` from the start of the
    /// buffer
    ///
    /// Preconditions: the buffer must be allocated and smaller than the offset, the element at
    /// the offset must be initialized, and there must be no other live references to the element
    pub unsafe fn get_unchecked_mut(&self, idx: usize) -> &mut T {
        #![allow(clippy::mut_from_ref)]
        unsafe { self.get_unchecked_raw(idx).as_mut_unchecked().assume_init_mut() }
    }

    /// Get a mutable reference to the _initialized_ element at `offset` from the start of the
    /// buffer
    ///
    /// Preconditions: the buffer must be allocated and smaller than the offset, the element at
    /// the offset must be initialized, and there must be no other live mutable references to the
    /// element
    pub unsafe fn get_unchecked_ref(&self, idx: usize) -> &T {
        unsafe { self.get_unchecked_raw(idx).as_mut_unchecked().assume_init_ref() }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        if let Some(buf) = self.buf.get() {
            unsafe {
                let slc = std::slice::from_raw_parts_mut(buf.as_ptr(), self.cap.get());
                std::mem::drop(Box::from_raw(slc));
            }
        }
    }
}
