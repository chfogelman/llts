use std::cell::{Cell, UnsafeCell};
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::Drop;

pub struct UnmutQueue<T> {
    buf:  Box<[UnsafeCell<MaybeUninit<T>>]>,
    head: Cell<usize>,
    len:  Cell<usize>,
}

impl<T> UnmutQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf:  unsafe { Box::new_uninit_slice(capacity).assume_init() },
            head: Cell::new(0),
            len:  Cell::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, elem: T) {
        if self.len() == self.capacity() {
            panic!("Cannot push to CellQueue; length {} equals capacity", self.len());
        }
        let idx = (self.head.get() + self.len()) % self.capacity();
        unsafe {
            self.buf[idx].get().write(MaybeUninit::new(elem));
        }
        self.len.set(self.len.get() + 1);
    }

    pub fn pop(&self) -> Option<T> {
        if self.len() == 0 {
            None
        } else {
            let idx = self.head.get();
            let val = unsafe {
                // SAFETY: because the length is not 0, we know that the value at index `head` must
                // be initialized.
                self.buf[idx].get().read().assume_init_read()
            };
            self.len.set(self.len.get() - 1);
            self.head.set((self.head.get() + 1) % self.capacity());
            Some(val)
        }
    }

    pub fn drain(&self) -> Drain<'_, T> {
        Drain(self)
    }
}

impl<T> FromIterator<T> for UnmutQueue<T> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let iter = iter.into_iter();
        let q = Self::new(iter.size_hint().1.unwrap_or(0));
        for item in iter {
            q.push(item)
        }
        q
    }
}

impl<T> Drop for UnmutQueue<T> {
    fn drop(&mut self) {
        for i in self.head.get()..(self.head.get() + self.len()) {
            let idx = i % self.capacity();
            unsafe {
                // SAFETY: because this element is between `head` and `head + len`, we are
                // guaranteed that it is initialized and now needs to be dropped.
                self.buf[idx].get().read().assume_init_drop();
            }
        }
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
