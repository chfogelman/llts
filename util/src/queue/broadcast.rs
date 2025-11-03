use std::cell::Cell;
#[cfg(not(miri))]
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;
#[cfg(miri)]
use std::sync::Mutex;
use std::sync::atomic::{self, AtomicU64, Ordering};

use crate::align::CacheLineAligned;

/// Errors that can be returned by [`Receiver::recv`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecvError {
    /// Indicates that this [`Receiver`] has lagged behind the [`Sender`].
    Lagged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    /// Indicates that this [`Receiver`] has lagged behind the [`Sender`].
    Lagged,
    /// Indicates that the queue is empty.
    Empty,
}

#[repr(align(64))]
struct Block<T> {
    seqno: AtomicU64,
    #[cfg(not(miri))]
    val:   UnsafeCell<MaybeUninit<T>>,
    #[cfg(miri)]
    val:   Mutex<MaybeUninit<T>>,
}

// SAFETY: Blocks are "synchronized" by their seqno.
unsafe impl<T: Copy> Sync for Block<T> {}

impl<T> Default for Block<T> {
    fn default() -> Self {
        Block {
            seqno: AtomicU64::new(0),
            #[cfg(not(miri))]
            val: UnsafeCell::new(MaybeUninit::uninit()),
            #[cfg(miri)]
            val: Mutex::new(MaybeUninit::uninit()),
        }
    }
}

struct QueueImpl<T> {
    /// This is how many blocks the writer has written. Some invariants:
    /// block_counter % items.len() is the next position the writer will write to.
    /// block_counter * 2 is the sequence number of the previously written block, if any
    /// A receiver can use these properties to catch up with the sender if it falls behind.
    block_counter: CacheLineAligned<AtomicU64>,
    items:         Box<[Block<T>]>,
}

/// The sending side of a [`channel`].
///
/// `Sender`s cannot be cloned and do not implement [`Sync`]. However, they can be sent between
/// threads safely. One sender broadcasts messages to all the receivers for this queue.
pub struct Sender<T> {
    queue: Arc<QueueImpl<T>>,

    // Important not only that there is only one sender, but also that it is not accidentally
    // shared between threads. `usize` is Send but not Sync, so it's appropriate to prevent
    // automatic `Sync` implementation
    _unsync: PhantomData<usize>,
}

impl<T> Sender<T> {
    fn new(queue: Arc<QueueImpl<T>>) -> Self {
        Self {
            queue,
            _unsync: PhantomData,
        }
    }
}

impl<T: Copy> Sender<T> {
    /// Inserts `val` at the current head of the queue. This may cause a consumer to lag
    pub fn send(&self, val: T) {
        let position = self.queue.block_counter.load(Ordering::Relaxed);

        // SAFETY: We asserted in `channel` that the queue's capacity is greater than 1
        unsafe { std::hint::assert_unchecked(self.queue.items.len() > 0) }

        let block = &self.queue.items[position as usize & (self.queue.items.len() - 1)];
        block.seqno.store(position * 2 + 1, Ordering::Relaxed);

        // Ensure the value write is not reordered before the store to seqno.
        atomic::fence(Ordering::Release);
        #[cfg(miri)]
        block.val.lock().unwrap().write(val);

        // SAFETY: technically, none. write_volatile is not atomic. However, seqlock semantics
        // ensure that the reader never treats torn reads as valid.
        #[cfg(not(miri))]
        unsafe {
            std::ptr::write_volatile(block.val.get() as *mut _, MaybeUninit::new(val));
        }
        block.seqno.store(position * 2 + 2, Ordering::Release);
        self.queue.block_counter.store(position + 1, Ordering::Relaxed);
    }
}

/// The receiver side of a [`channel`].
///
/// NOTE: A `Receiver` is cloneable, but be aware that the resulting clone will start receiving
/// from the same point in the queue as its source. If the source has already received some values,
/// the clone will not be able to receive those same values.
#[derive(Clone)]
pub struct Receiver<T> {
    queue:       Arc<QueueImpl<T>>,
    block_count: Cell<u64>,
}

impl<T> Receiver<T> {
    fn new(queue: Arc<QueueImpl<T>>) -> Self {
        Self {
            queue,
            block_count: Cell::new(0),
        }
    }
}

impl<T: Copy> Receiver<T> {
    /// Receives the next item in the queue. This function will block the thread in a spin-loop
    /// until the next item is available. This will return [`RecvError::Lagged`] if the producer
    /// has gotten ahead of this Receiver. The next value received after lagging will be the value
    /// most recently sent by the producer.
    pub fn recv(&self) -> Result<T, RecvError> {
        Self::recv_impl::<RecvError, true>(self, || unreachable!(), || RecvError::Lagged)
    }

    /// Receives the next item in the queue, if any. This will return [`TryRecvError::Lagged`] if the
    /// producer has gotten ahead of this Receiver. The next value received after lagging will be
    /// the value most recently sent by the producer.
    ///
    /// If the queue is empty, [`TryRecvError::Empty`] will be returned. If spinning on this return
    /// value, it is recommended to use [`Receiver::recv`] instead, or employ
    /// [`std::hint::spin_loop()`].
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        Self::recv_impl::<TryRecvError, false>(self, || TryRecvError::Empty, || TryRecvError::Lagged)
    }

    // Helper for implementing `recv` and `try_recv`
    #[inline]
    fn recv_impl<Error, const BLOCKING: bool>(
        &self,
        make_empty_err: impl Fn() -> Error,
        make_lagged_err: impl Fn() -> Error,
    ) -> Result<T, Error> {
        let block_count = self.block_count.get();
        // SAFETY: We asserted in `channel` that the queue's capacity is greater than 1
        unsafe { std::hint::assert_unchecked(self.queue.items.len() > 0) }

        let block = &self.queue.items[block_count as usize & (self.queue.items.len() - 1)];
        let our_seqno = block_count * 2 + 2;
        if const { BLOCKING } {
            while our_seqno > block.seqno.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        } else {
            if our_seqno > block.seqno.load(Ordering::Relaxed) {
                return Err(make_empty_err());
            }
        }

        loop {
            let seqno1 = block.seqno.load(Ordering::Acquire);
            if seqno1 & 0x1 == 1 {
                continue;
            }
            #[cfg(miri)]
            let maybe_val = block.val.lock().unwrap();

            // SAFETY: Technically, none. read_volatile is not atomic. However, seqlock semantics
            // ensure that we will not call .assume_init() on torn values.
            #[cfg(not(miri))]
            let maybe_val = unsafe { std::ptr::read_volatile(block.val.get()) };
            atomic::fence(Ordering::Acquire);

            let seqno2 = block.seqno.load(Ordering::Acquire);
            if seqno1 == seqno2 {
                if seqno1 == our_seqno {
                    self.block_count.set(block_count + 1);
                    // SAFETY: At this point, we can be sure that we have read a value that was
                    // indeed previously written, and that it is not torn.
                    return unsafe { Ok(maybe_val.assume_init()) };
                } else {
                    self.block_count.set(self.queue.block_counter.load(Ordering::Relaxed));
                    return Err(make_lagged_err());
                }
            }
        }
    }
}

/// Creates a new broadcast channel suitable for sending from a single producer to multiple consumers.
///
/// All consumers will receive the same sequence of messages, as long as they do not lag behind the
/// producer. The [`Sender`] cannot be cloned, but the [`Receiver`] may be cloned. Sending will
/// never result in an error, and receivers can detect when they have lagged behind the sender. See
/// [`RecvError`] and [`TryRecvError`] for details.
///
/// The capacity of the queue is fixed at construction and cannot be changed. It must be a power of
/// two and greater than 1.
///
/// #Examples
///
/// ```
/// use util::queue::broadcast;
///
/// let (tx, rx1) = broadcast::channel(1 << 3);
/// let rx2 = rx1.clone();
///
/// // Thread 0
/// tx.send(42);
///
/// // Thread 1
/// assert_eq!(rx1.recv().unwrap(), 42);
///
/// // Thread 2
/// assert_eq!(rx2.recv().unwrap(), 42);
///
#[must_use]
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    assert!(capacity > 1, "Broadcast queue capacity must be greater than 1");
    assert!(capacity.count_ones() == 1, "Broadcast queue capacity must be a power of 2");

    let items = {
        let mut items = Box::new_uninit_slice(capacity);
        items.iter_mut().for_each(|val| {
            val.write(Default::default());
        });
        unsafe { items.assume_init() }
    };
    let queue = Arc::new(QueueImpl {
        block_counter: CacheLineAligned(AtomicU64::new(0)),
        items,
    });
    (Sender::new(queue.clone()), Receiver::new(queue))
}
