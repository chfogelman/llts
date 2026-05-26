use std::cell::{Cell, UnsafeCell};
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::Drop;
use std::ptr::NonNull;

/// A queue that does not require a mutable reference for pushing and popping.
///
/// This is useful when you need a queue that is shared between two owners on the same thread, and
/// you can't (or don't want to) use [`std::cell::RefCell`]. The tradeoff here is that this
/// container does not implement [`Sync`], and you cannot safely obtain references to the individual
/// elements in the queue. The operations are generally limited to variations of pushing and popping only.
pub struct UnmutQueue<T> {
    buf:  Cell<Option<NonNull<UnsafeCell<MaybeUninit<T>>>>>,
    cap:  Cell<usize>,
    head: Cell<usize>,
    len:  Cell<usize>,
}

impl<T> UnmutQueue<T> {
    pub const fn new() -> Self {
        Self {
            buf:  Cell::new(None),
            cap:  Cell::new(0),
            head: Cell::new(0),
            len:  Cell::new(0),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (buf, cap) = if capacity == 0 {
            (None, 0)
        } else {
            let capacity = if capacity.count_ones() == 1 {
                capacity
            } else {
                1 << (capacity.ilog2() + 1)
            };
            let mem = unsafe {
                // SAFETY: a [MaybeUninit<UnsafeCell<MaybeUninit<T>>>] is as initialized as it
                // needs to be.
                Box::leak(Box::new_uninit_slice(capacity).assume_init())
            };
            (NonNull::new(mem.as_mut_ptr()), mem.len())
        };
        Self {
            buf:  Cell::new(buf),
            cap:  Cell::new(cap),
            head: Cell::new(0),
            len:  Cell::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.cap.get()
    }

    fn mask(&self) -> usize {
        debug_assert_eq!(self.cap.get().count_ones(), 1);
        self.capacity() - 1
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, elem: T) {
        unsafe {
            self.realloc_for_push();
            self.push_unchecked(elem);
        }
    }

    pub fn try_push(&self, elem: T) -> Result<(), T> {
        if self.len() == self.capacity() {
            Err(elem)
        } else {
            unsafe {
                // SAFETY: There is guaranteed to be enough space in the queue to push here.
                self.push_unchecked(elem);
            }
            Ok(())
        }
    }

    pub fn pop(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let offset = self.head.get();
            let val = unsafe {
                // SAFETY: because the length is not 0, we know that the value at index `head` must
                // be initialized.
                self.get_raw(offset).read().assume_init()
            };
            self.len.set(self.len.get() - 1);
            self.head.set((self.head.get() + 1) & self.mask());
            Some(val)
        }
    }

    pub fn drain(&self) -> Drain<'_, T> {
        Drain(self)
    }

    pub fn swap(&self, other: &Self) {
        self.buf.swap(&other.buf);
        self.cap.swap(&other.cap);
        self.len.swap(&other.len);
        self.head.swap(&other.head);
    }

    unsafe fn realloc_for_push(&self) {
        debug_assert!(self.len() <= self.capacity(), "Invariant violation: length should not exceed capacity");
        if self.len() == self.capacity() {
            let new_capacity = if self.capacity() > 0 {
                debug_assert!(self.buf.get().is_some());
                2 * self.capacity()
            } else {
                debug_assert!(self.buf.get().is_none());
                1
            };
            let other = Self::with_capacity(new_capacity);
            while let Some(elem) = self.pop() {
                unsafe {
                    // SAFETY: we have just constructed `other` with a capacity larger than
                    // `self`'s.
                    other.push_unchecked(elem)
                }
            }
            self.swap(&other);
        }
    }

    /// Push an element to the queue without checking the queue's current capacity.
    ///
    /// Precondition: the queue must have an allocated buffer whose capacity is smaller than the
    /// current number of stored elements.
    unsafe fn push_unchecked(&self, elem: T) {
        debug_assert!(self.len() < self.capacity());
        debug_assert!(self.buf.get().is_some());
        let offset = (self.head.get() + self.len()) & self.mask();
        unsafe {
            self.get_raw(offset).write(MaybeUninit::new(elem));
        }
        self.len.set(self.len.get() + 1);
    }

    /// Get a pointer to the element at `offset` from the start of the queue's underlying buffer.
    ///
    /// Preconditions: the buffer must be allocated and smaller than the offset
    unsafe fn get_raw(&self, offset: usize) -> *mut MaybeUninit<T> {
        debug_assert!(self.buf.get().is_some());
        debug_assert!(offset < self.capacity());
        unsafe { self.buf.get().unwrap_unchecked().add(offset).as_ref().get() }
    }
}

impl<T: Clone> Clone for UnmutQueue<T> {
    fn clone(&self) -> Self {
        let new_q = Self::with_capacity(self.len());
        for i in self.head.get()..(self.head.get() + self.len()) {
            let offset = i & self.mask();
            unsafe {
                let cloned = self.get_raw(offset).as_ref_unchecked().assume_init_ref().clone();
                new_q.push_unchecked(cloned);
            };
        }
        new_q
    }
}

impl<T> Drop for UnmutQueue<T> {
    fn drop(&mut self) {
        if let Some(buf) = self.buf.get() {
            for i in self.head.get()..(self.head.get() + self.len()) {
                let offset = i & self.mask();
                unsafe {
                    // SAFETY: because this element is between `head` and `head + len`, we are
                    // guaranteed that it is initialized and now needs to be dropped.
                    self.get_raw(offset).as_mut_unchecked().assume_init_drop()
                }
            }
            unsafe {
                let slc = std::slice::from_raw_parts_mut(buf.as_ptr(), self.cap.get());
                std::mem::drop(Box::from_raw(slc));
            }
        }
    }
}

impl<T> FromIterator<T> for UnmutQueue<T> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let iter = iter.into_iter();
        let q = Self::with_capacity(iter.size_hint().1.unwrap_or(0));
        for item in iter {
            q.push(item)
        }
        q
    }
}

pub struct Iter<T>(UnmutQueue<T>);

impl<T> Iterator for Iter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl<T> FusedIterator for Iter<T> {}

pub struct Drain<'a, T>(&'a UnmutQueue<T>);

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl<'a, T> FusedIterator for Drain<'a, T> {}

impl<T> IntoIterator for UnmutQueue<T> {
    type Item = T;

    type IntoIter = Iter<T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self)
    }
}

#[cfg(test)]
mod test;
