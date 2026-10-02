//! A single-producer, multi-consumer broadcast queue.
//!
//! All consumers will receive the same set of messages, which must implement [`Copy`] for this
//! queue to be of any use. If a consumer lags so far behind the producer that it starts dropping
//! messages, this will be indicated via [`Lagged`], and the [`Receiver`] handle will
//! set itself up to receive the most recently written message. This _does_ result in lost messages,
//! so this queue must be combined with proper recovery mechanisms.

use std::cell::Cell;
#[cfg(not(miri))]
use std::cell::UnsafeCell;
use std::error::Error;
use std::fmt::Display;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;
#[cfg(miri)]
use std::sync::Mutex;
use std::sync::atomic::{self, AtomicU64, Ordering};

/// Error that indicates invalid capacity passed to [`channel`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacityError(usize);

impl Display for CapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid capacity {} to broadcast::channel; must be power of two and greater than 1",
            self.0
        )
    }
}

impl Error for CapacityError {}

/// Indicates that this [`Receiver`] has lagged behind the [`Sender`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lagged;

impl Display for Lagged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("broadcast queue Receiver lagged")
    }
}

impl Error for Lagged {}

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
        Self {
            seqno: AtomicU64::new(0),
            #[cfg(not(miri))]
            val: UnsafeCell::new(MaybeUninit::uninit()),
            #[cfg(miri)]
            val: Mutex::new(MaybeUninit::uninit()),
        }
    }
}

struct QueueBuf<T> {
    storage: Arc<[Block<T>]>,
}

impl<T> Clone for QueueBuf<T> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

impl<T> QueueBuf<T> {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 1, "Broadcast queue capacity must be greater than 1");
        assert!(capacity.is_power_of_two(), "Broadcast queue capacity must be a power of 2");

        // HACK: We must allocate an additional block at the end,
        // whose `seqno` is abused to provide the block count
        let mut storage = Arc::new_uninit_slice(capacity + 1);
        for val in Arc::get_mut(&mut storage).expect("Cannot have been cloned") {
            val.write(Block::default());
        }
        let storage = unsafe {
            // SAFETY: we have just written the blocks above
            storage.assume_init()
        };
        Self { storage }
    }

    fn len(&self) -> usize {
        self.storage.len() - 1
    }

    fn get(&self, position: usize) -> &Block<T> {
        debug_assert!(
            (position & (self.len() - 1)) < self.len(),
            "Invariant violation, length may not be power of 2"
        );
        unsafe {
            // SAFETY: We have asserted on construction that the buffer was not constructed with
            // capacity 0.
            self.storage.get_unchecked(position & (self.len() - 1))
        }
    }

    /// This is how many blocks the writer has written. Some invariants:
    /// `write_count() % len()` is the next position the writer will write to.
    /// `write_count() * 2` is the sequence number of the previously written block, if any
    /// A receiver can use these properties to catch up with the sender if it falls behind.
    fn write_count(&self) -> &AtomicU64 {
        // HACK: an extra block in the storage array is (ab)used to hold the block count.
        // This allows us to store the block count contiguously with the queue items, but
        // uses more memory than strictly necessary.
        &self.storage[self.storage.len() - 1].seqno
    }
}

/// The sending side of a [`channel`].
///
/// `Sender`s cannot be cloned and do not implement [`Sync`]. However, they can be sent between
/// threads safely. One sender broadcasts messages to all the receivers for this queue.
pub struct Sender<T> {
    queue: QueueBuf<T>,

    // Important not only that there is only one sender, but also that it is not accidentally
    // shared between threads. `usize` is Send but not Sync, so it's appropriate to prevent
    // automatic `Sync` implementation
    _unsync: PhantomData<usize>,
}

impl<T> Sender<T> {
    fn new(queue: QueueBuf<T>) -> Self {
        Self {
            queue,
            _unsync: PhantomData,
        }
    }
}

impl<T: Copy> Sender<T> {
    /// Inserts `val` at the current head of the queue. This may cause a consumer to lag
    pub fn send(&self, val: T) {
        let position = self.queue.write_count().load(Ordering::Relaxed);

        let block = &self.queue.get(position as usize);
        block.seqno.store(position * 2 + 1, Ordering::Relaxed);

        // Ensure the value write is not reordered before the store to seqno.
        atomic::fence(Ordering::Release);
        #[cfg(miri)]
        block.val.lock().expect("wtf how lol").write(val);

        #[cfg(not(miri))]
        unsafe {
            // SAFETY: technically, none. write_volatile is not atomic. However, seqlock semantics
            // ensure that the reader never treats torn reads as valid.
            std::ptr::write_volatile(block.val.get().cast(), MaybeUninit::new(val));
        }
        block.seqno.store(position * 2 + 2, Ordering::Release);
        self.queue.write_count().store(position + 1, Ordering::Relaxed);
    }
}

/// The receiver side of a [`channel`].
///
/// NOTE: A `Receiver` is cloneable, but be aware that the resulting clone will start receiving
/// from the same point in the queue as its source. If the source has already received some values,
/// the clone will not be able to receive those same values.
#[derive(Clone)]
pub struct Receiver<T> {
    queue:      QueueBuf<T>,
    // `read_count` is intentionally a Cell type and _not_ an atomic type.
    // A [`Receiver`] cannot be shared between threads. It must be cloned.
    read_count: Cell<u64>,
}

impl<T> Receiver<T> {
    fn new(queue: QueueBuf<T>) -> Self {
        Self {
            queue,
            read_count: Cell::new(0),
        }
    }
}

impl<T: Copy> Receiver<T> {
    /// Receives the next item in the queue. This function will block the thread in a spin-loop
    /// until the next item is available. This will return [`Lagged`] if the producer
    /// has gotten ahead of this Receiver. The next value received after lagging will be the value
    /// most recently sent by the producer.
    pub fn recv(&self) -> Result<T, Lagged> {
        unsafe {
            // SAFETY: Passing `true` ensures this function blocks until there is a value available
            self.recv_impl::<true>().unwrap_unchecked()
        }
    }

    /// Receives the next item in the queue, if any, or [`Lagged`] if the producer has gotten
    /// ahead of this Receiver. The next value received after lagging will be the value most
    /// recently sent by the producer.
    ///
    /// If the queue is empty, [`None`] will be returned. If spinning on this return
    /// value, it is recommended to use [`Receiver::recv`] instead, or employ [`std::hint::spin_loop()`].
    pub fn try_recv(&self) -> Option<Result<T, Lagged>> {
        self.recv_impl::<false>()
    }

    // Helper for implementing `recv` and `try_recv`
    #[inline]
    fn recv_impl<const BLOCKING: bool>(&self) -> Option<Result<T, Lagged>> {
        let read_count = self.read_count.get();

        let block = &self.queue.get(read_count as usize);
        let our_seqno = read_count * 2 + 2;
        if const { BLOCKING } {
            while our_seqno > block.seqno.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        } else if our_seqno > block.seqno.load(Ordering::Relaxed) {
            return None;
        }

        loop {
            let seqno1 = block.seqno.load(Ordering::Acquire);
            if seqno1 & 0x1 == 1 {
                continue;
            }
            #[cfg(miri)]
            let maybe_val = block.val.lock().expect("wtf how actually");

            #[cfg(not(miri))]
            let maybe_val = unsafe {
                // SAFETY: Technically, none. read_volatile is not atomic. However, seqlock semantics
                // ensure that we will not call .assume_init() on torn values.
                std::ptr::read_volatile(block.val.get())
            };
            atomic::fence(Ordering::Acquire);

            let seqno2 = block.seqno.load(Ordering::Relaxed);
            if seqno1 == seqno2 {
                if seqno1 == our_seqno {
                    self.read_count.set(read_count + 1);
                    // SAFETY: At this point, we can be sure that we have read a value that was
                    // indeed previously written, and that it is not torn.
                    return unsafe { Some(Ok(maybe_val.assume_init())) };
                } else {
                    // Catch up with the producer
                    self.read_count.set(self.queue.write_count().load(Ordering::Relaxed));
                    return Some(Err(Lagged));
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
/// [`Lagged`].
///
/// The capacity of the queue is fixed at construction and cannot be changed. It must be a power of
/// two and greater than 1.
///
/// # Examples
///
/// ```
/// use util::queue::broadcast;
///
/// let (tx, rx1) = broadcast::channel(1 << 3)
///     .expect("Capacity must be a power of two and greater than 1");
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
#[must_use = "Returned channel must be used"]
pub fn channel<T>(capacity: usize) -> Result<(Sender<T>, Receiver<T>), CapacityError> {
    if capacity <= 1 || capacity.count_ones() != 1 {
        Err(CapacityError(capacity))
    } else {
        let queue = QueueBuf::new(capacity);
        Ok((Sender::new(queue.clone()), Receiver::new(queue)))
    }
}
