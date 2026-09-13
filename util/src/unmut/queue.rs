use std::cell::Cell;
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::Drop;

use crate::unmut::buffer::Buffer;

/// A queue that does not require a mutable reference for pushing and popping.
///
/// This is useful when you need a queue that is shared between two owners on the same thread, and
/// you can't (or don't want to) use [`std::cell::RefCell`]. The tradeoff is that this
/// container does not implement [`Sync`], and you cannot safely obtain references to the individual
/// elements in the queue. The operations are generally limited to variations of pushing and popping only.
pub struct Queue<T> {
    buf:  Buffer<T>,
    head: Cell<usize>,
    len:  Cell<usize>,
}

impl<T> Queue<T> {
    pub const fn new() -> Self {
        Self {
            buf:  Buffer::new(),
            head: Cell::new(0),
            len:  Cell::new(0),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = if capacity == 0 || capacity.count_ones() == 1 {
            capacity
        } else {
            1 << (capacity.ilog2() + 1)
        };
        Self {
            buf:  Buffer::with_capacity(cap),
            head: Cell::new(0),
            len:  Cell::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    fn mask(&self) -> usize {
        debug_assert_eq!(self.capacity().count_ones(), 1);
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
                self.buf.get_unchecked_raw(offset).read().assume_init()
            };
            self.len.set(self.len.get() - 1);
            self.head.set((self.head.get() + 1) & self.mask());
            Some(val)
        }
    }

    pub fn pop_if(&self, pred: impl FnOnce(&T) -> bool) -> Option<T> {
        // SAFETY: As long as the queue is not empty, it is safe to inspect the head
        if !self.is_empty() && unsafe { pred(self.buf.get_unchecked_ref(self.head.get())) } {
            self.pop()
        } else {
            None
        }
    }

    pub fn drain(&self) -> Drain<'_, T> {
        Drain(self)
    }

    pub fn swap(&self, other: &Self) {
        self.buf.swap(&other.buf);
        self.len.swap(&other.len);
        self.head.swap(&other.head);
    }

    pub fn clear(&self) {
        for i in self.head.get()..(self.head.get() + self.len()) {
            let offset = i & self.mask();
            unsafe {
                // SAFETY: because this element is between `head` and `head + len`, we are
                // guaranteed that it is initialized and can be dropped.
                std::ptr::drop_in_place(self.buf.get_unchecked_mut(offset))
            }
        }
        self.len.set(0)
    }

    unsafe fn realloc_for_push(&self) {
        debug_assert!(self.len() <= self.capacity(), "Invariant violation: length should not exceed capacity");
        if self.len() < self.capacity() {
            return;
        }
        let new_capacity = if self.capacity() > 0 {
            debug_assert!(self.capacity().count_ones() == 1);
            2 * self.capacity()
        } else {
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

    /// Push an element to the queue without checking the queue's current capacity.
    ///
    /// Precondition: the queue must have an allocated buffer whose capacity is smaller than the
    /// current number of stored elements.
    unsafe fn push_unchecked(&self, elem: T) {
        debug_assert!(self.len() < self.capacity());
        let offset = (self.head.get() + self.len()) & self.mask();
        unsafe {
            self.buf.get_unchecked_raw(offset).write(MaybeUninit::new(elem));
        }
        self.len.set(self.len.get() + 1);
    }
}

impl<T: Clone> Clone for Queue<T> {
    fn clone(&self) -> Self {
        let new_q = Self::with_capacity(self.len());
        for i in self.head.get()..(self.head.get() + self.len()) {
            let offset = i & self.mask();
            unsafe {
                let cloned = self.buf.get_unchecked_ref(offset).clone();
                new_q.push_unchecked(cloned);
            };
        }
        new_q
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        self.clear()
    }
}

impl<T> FromIterator<T> for Queue<T> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let iter = iter.into_iter();
        let q = Self::with_capacity(iter.size_hint().1.unwrap_or(0));
        for item in iter {
            q.push(item)
        }
        q
    }
}

pub struct Iter<T>(Queue<T>);

impl<T> Iterator for Iter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl<T> FusedIterator for Iter<T> {}

pub struct Drain<'a, T>(&'a Queue<T>);

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        self.0.clear()
    }
}

impl<'a, T> FusedIterator for Drain<'a, T> {}

impl<T> IntoIterator for Queue<T> {
    type Item = T;

    type IntoIter = Iter<T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self)
    }
}

#[cfg(test)]
mod test;
