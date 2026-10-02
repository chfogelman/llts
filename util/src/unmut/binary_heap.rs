use std::cell::Cell;
use std::mem::MaybeUninit;
use std::ops::Drop;

use crate::unmut::buffer::Buffer;

/// A max binary heap that does not require a mutable reference for pushing and popping.
///
/// This is useful when you need a heap that is shared between two owners on the same thread, and
/// you can't (or don't want to) use [`std::cell::RefCell`]. The tradeoff is that this
/// container does not implement [`Sync`], and you cannot safely obtain references to the individual
/// elements in the heap. The operations are generally limited to variations of pushing and popping only.
pub struct BinaryHeap<T> {
    buf: Buffer<T>,
    len: Cell<usize>,
}

impl<T> BinaryHeap<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: Buffer::new(),
            len: Cell::new(0),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Buffer::with_capacity(capacity),
            len: Cell::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len.get() == 0
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    fn swap(&self, other: &Self) {
        self.buf.swap(&other.buf);
        self.len.swap(&other.len);
    }

    fn realloc_for_push(&self) {
        debug_assert!(self.len() <= self.capacity(), "Invariant violation: length should not exceed capacity");
        if self.len() < self.capacity() {
            return;
        }
        if self.capacity() == 0 {
            self.swap(&Self::with_capacity(1));
            return;
        }
        let new_capacity = self
            .capacity()
            .checked_next_power_of_two()
            .expect("next power of two to be <= usize::MAX")
            .checked_mul(2)
            .expect("next power-of-two capacity to be <= usize::MAX");
        let other = Self::with_capacity(new_capacity);
        unsafe {
            // SAFETY: at this point, both heaps' buffers have been allocated, and are not
            // overlapping.
            std::ptr::copy_nonoverlapping(self.buf.as_mut_ptr(), other.buf.as_mut_ptr(), self.len());
            // SAFETY: This is not technically unsafe, but other now holds self's elements, so do
            // the lenth swap here
            other.len.swap(&self.len);
        }
        self.swap(&other);
    }

    pub fn clear(&self) {
        for i in 0..self.len() {
            unsafe {
                // SAFETY: self.len() is the upper bound on initialized elements
                std::ptr::drop_in_place(self.buf.get_unchecked_mut(i));
            }
        }
        self.len.set(0);
    }
}

impl<T> Default for BinaryHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> BinaryHeap<T> {
    /// Pushes an item to the heap, reallocating if necessary
    pub fn push(&self, item: T) {
        self.realloc_for_push();
        unsafe {
            // SAFETY: We have just realloc'ed
            self.push_unchecked(item);
        }
    }

    /// Pushes an item to the heap if there's available capacity
    ///
    /// # Errors
    /// Returns the item if the heap is full
    pub fn try_push(&self, item: T) -> Result<(), T> {
        if self.len() == self.capacity() {
            self.push(item);
            Ok(())
        } else {
            Err(item)
        }
    }

    /// Pops an item from the heap, if nonempty
    pub fn pop(&self) -> Option<T> {
        debug_assert!(self.len() <= self.capacity(), "Invariant violation: length exceeds capacity");
        if self.is_empty() {
            return None;
        }
        self.len.set(self.len() - 1);
        let val = unsafe {
            // SAFETY: we cannot reach this point if the queue is empty
            self.buf.get_unchecked_raw(0).read().assume_init()
        };
        // There is now an empty hole at index 0, and the length is correct.
        // we only need to fill the hole and heapify if it was not the last element
        if !self.is_empty() {
            unsafe {
                // SAFETY: the buffer is guaranteed non-empty
                self.buf
                    .get_unchecked_raw(0)
                    .write(self.buf.get_unchecked_raw(self.len()).read());
                self.sift_down(0);
            }
        }
        Some(val)
    }

    /// Pop the maximum element if it exists and the predicate returns true
    ///
    /// # Safety
    /// The caller must ensure that calls to `pred` cannot modify the binary heap while a reference
    /// to the first element is held.
    pub unsafe fn pop_if(&self, pred: impl FnOnce(&T) -> bool) -> Option<T> {
        if !self.is_empty()
            && unsafe {
                // SAFETY: inside unsafe fn
                pred(self.buf.get_unchecked_ref(0))
            }
        {
            self.pop()
        } else {
            None
        }
    }

    unsafe fn push_unchecked(&self, item: T) {
        debug_assert!(self.len() < self.capacity(), "Invariant violation: pushing without available capacity");
        let idx = self.len();
        unsafe {
            // SAFETY: the precondition self.len() < self.capacity() is met, so we can write to
            // index self.len, then increase the length
            self.buf.get_unchecked_raw(idx).write(MaybeUninit::new(item));
            self.len.set(self.len() + 1);

            // SAFETY: the element at idx is now initialized, so we can sift it up
            self.sift_up(idx);
        }
    }

    unsafe fn sift_up(&self, mut idx: usize) {
        unsafe {
            // SAFETY: idx != 0 implies idx / 2 != idx. Thus they do not overlap, and it is safe to
            // take references to them within the bounds of this loop.
            while idx > 0 && self.buf.get_unchecked_ref(idx / 2) < self.buf.get_unchecked_ref(idx) {
                std::mem::swap(self.buf.get_unchecked_mut(idx / 2), self.buf.get_unchecked_mut(idx));
                idx /= 2;
            }
        }
    }

    unsafe fn sift_down(&self, mut largest: usize) {
        loop {
            let left = (largest * 2) + 1;
            let right = (largest + 1) * 2;
            let mut target = largest;
            unsafe {
                // SAFETY: target != left
                if left < self.len() && self.buf.get_unchecked_ref(target) < self.buf.get_unchecked_ref(left) {
                    target = left;
                }
            }
            unsafe {
                // SAFETY: target == left or target == largest, but target != right
                if right < self.len() && self.buf.get_unchecked_ref(target) < self.buf.get_unchecked_ref(right) {
                    target = right;
                }
            }
            if target == largest {
                break;
            }
            unsafe {
                // SAFETY: target obvs != largest
                std::mem::swap(self.buf.get_unchecked_mut(target), self.buf.get_unchecked_mut(largest));
            }
            largest = target;
        }
    }
}

impl<T> Drop for BinaryHeap<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod test;
