use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::align::CacheLineAligned;

const INDEX_MASK: u8 = 0b011;
const DIRTY_MASK: u8 = 0b100;

// Stores the current state. These fields can be on the same cache line because only `flagged_idx`
// is updated regularly.
struct State {
    is_acquired: AtomicBool,
    flagged_idx: AtomicU8,
}

pub struct TripleBuffer<T> {
    state: CacheLineAligned<State>,
    slots: [CacheLineAligned<UnsafeCell<T>>; 3],
}

impl<T: Clone> TripleBuffer<T> {
    pub fn new(init: T) -> Self {
        let state = State {
            is_acquired: false.into(),
            flagged_idx: 0.into(),
        };
        Self {
            state: CacheLineAligned(state),
            slots: [init.clone(), init.clone(), init].map(|x| CacheLineAligned(UnsafeCell::new(x))),
        }
    }
}

// SAFETY: We guarantee soundness via the typical triple-buffering scheme.
// The reader controls one buffer, and the writer controls two.
// When the reader wants a new value, it swaps
unsafe impl<T> Sync for TripleBuffer<T> {}

pub struct Reader<P> {
    tb:      P,
    current: u8,
}

impl<T, P: Deref<Target = TripleBuffer<T>>> Reader<P> {
    /// Obtain a reference to the latest value
    pub fn read(&mut self) -> &T {
        self.read_if().0
    }

    /// Obtain a reference to the latest value if it is newer than the last read value
    pub fn read_newer(&mut self) -> Option<&T> {
        let (val, flag) = self.read_if();
        flag.then_some(val)
    }

    fn read_if(&mut self) -> (&T, bool) {
        // `current` must not be made `Cell<usize>` and allow this function to take `self` mutably.
        // Doing so allows obtaining references to two different buffers in the array.
        let flagged_idx = self.tb.state.flagged_idx.load(Ordering::Acquire);
        let (idx, flag) = if flagged_idx & DIRTY_MASK != 0 {
            let next = self.tb.state.flagged_idx.swap(self.current, Ordering::AcqRel) & INDEX_MASK;
            debug_assert_ne!(self.current, next, "Invariant violation: current and next indexes are equal");
            self.current = next;
            (next, true)
        } else {
            debug_assert_ne!(
                self.current,
                flagged_idx & INDEX_MASK,
                "Invariant violation: received clean index equal to current"
            );
            (self.current, false)
        };
        unsafe {
            // SAFETY: We have just acquired an exclusive index to read from
            (&*self.tb.slots[idx as usize].get(), flag)
        }
    }
}

pub struct Writer<P> {
    tb:    P,
    spare: u8,
}

pub struct WriterHandle<'a, T> {
    val:    &'a mut T,
    latest: &'a AtomicU8,
    spare:  &'a mut u8,
}

impl<T> WriterHandle<'_, T> {
    /// Explicitly drops this `WriterHandle`
    pub fn commit(self) {}
}

impl<T> Deref for WriterHandle<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<T> DerefMut for WriterHandle<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val
    }
}

impl<T> Drop for WriterHandle<'_, T> {
    fn drop(&mut self) {
        // Set dirty flag
        *self.spare = self.latest.swap(*self.spare | 0b100, Ordering::AcqRel) & 0b11;
    }
}

impl<T, P: Deref<Target = TripleBuffer<T>>> Writer<P> {
    pub fn get_mut(&mut self) -> WriterHandle<'_, T> {
        let val = unsafe {
            // SAFETY: the spare slot belongs to the writer
            &mut *self.tb.slots[self.spare as usize].get()
        };
        WriterHandle {
            val,
            latest: &self.tb.state.flagged_idx,
            spare: &mut self.spare,
        }
    }
}

pub fn new<T, P: Deref<Target = TripleBuffer<T>> + Clone>(tb_ptr: P) -> (Writer<P>, Reader<P>) {
    let acquired = tb_ptr.state.is_acquired.swap(true, Ordering::AcqRel);
    assert!(!acquired, "Attempted to claim TripleBuffer instance with existing reader and writer");
    tb_ptr.state.flagged_idx.store(0, Ordering::Release);
    let writer = Writer {
        tb:    tb_ptr.clone(),
        spare: 1,
    };
    let reader = Reader {
        tb:      tb_ptr,
        current: 2,
    };
    (writer, reader)
}

#[cfg(test)]
mod test;
