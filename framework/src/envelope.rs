use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;

use crate::{Context, Pipeline};

/// Type-erased function for dispatching the contents of an envelope onto a thread
/// managing a handler chain.
type DispatchFn<T> = unsafe fn(&mut T, *const ());

const BUF_SIZE: usize = 24;

/// Byte buffer for storing a type-erased object inside an envelope
type EnvelopeBuf = [mem::MaybeUninit<u8>; BUF_SIZE];

/// Type-erased envelope for sending messages between threads.
///
/// It provides optimizations for small [`Copy`] types as well as [`Arc`]. You should not need to
/// use this type directly. Instead, use [`crate::publish`]. Because it may contain a type-erased
/// [`Arc`], it must _not_ implement [`Copy`] itself. The derived Clone implementation would also
/// be incorrect for this type. [`Envelope::dispatch_onto`] must consume the Envelope and drop the
/// contained message
pub struct Envelope<Dst> {
    dispatcher: DispatchFn<Dst>,
    buf:        EnvelopeBuf,
}

impl<Dst: Pipeline> Envelope<Dst> {
    pub(crate) fn dispatch_onto(self, thread: &mut Dst) {
        let Envelope { dispatcher, ref buf } = self;

        // SAFETY: We make sure to use the dispatcher on this message with the message itself. It
        // would only be UB to use the dispatcher of one message with the buffer of another.
        // NOTE: You may feel compelled to change `self` to `&self`. DON'T. The `dispatcher`
        // function is also responsible for dropping the type-erased message. This function must be
        // called only once for an envelope.
        unsafe { dispatcher(thread, buf.as_ptr() as *const ()) }
    }
}

// These are the [`ThreadLocalEnvelope`] types that are used to publish to handlers on the same
// thread. They are not converted to [`EnvelopeBuilder`]s until it is time to publish to other
// threads. We implement them as a wrapper over a reference so that it's cheap to pass them around
// in registers when calling [`Handler::handle`] on one thread. Only once it's time to allocate for
// the message and broadcast it to other threads do we need to take ownership of the message again.
#[repr(transparent)]
struct SyncEnvelope<'a, Msg: Send + Sync + 'static>(&'a mut mem::ManuallyDrop<Msg>);
#[repr(transparent)]
struct ArcEnvelope<'a, Msg: Send + Sync + 'static>(&'a mut mem::ManuallyDrop<Arc<Msg>>);
#[repr(transparent)]
pub(crate) struct CopyEnvelope<'a, Msg: Copy + Send + 'static>(&'a Msg);

const _: () = const { assert!(mem::size_of::<SyncEnvelope<[u8; 100]>>() == mem::size_of::<usize>()) };
const _: () = const { assert!(mem::size_of::<ArcEnvelope<[u8; 100]>>() == mem::size_of::<usize>()) };
const _: () = const { assert!(mem::size_of::<CopyEnvelope<[u8; 100]>>() == mem::size_of::<usize>()) };

// These are the [`EnvelopeBuilder`] types that may contain an allocated [`Arc`] that will be
// cloned when publishing to each other thread. These are only created after the contained message
// has been published to each thread-local [`Handler`].
#[repr(transparent)]
struct SyncEnvelopeBuilder<Msg>(Arc<Msg>);
#[repr(transparent)]
struct ArcEnvelopeBuilder<Msg>(Arc<Msg>);
#[repr(transparent)]
struct CopyEnvelopeBuilder<Msg>(Msg);

pub trait EnvelopeBuilder<Msg> {
    fn build_envelope<Dst: Pipeline>(&self) -> Envelope<Dst>;
}

impl<Msg: Send + Sync + 'static> EnvelopeBuilder<Msg> for SyncEnvelopeBuilder<Msg> {
    fn build_envelope<Dst: Pipeline>(&self) -> Envelope<Dst> {
        unsafe fn sync_dispatcher<Dst: Pipeline, Msg>(ctx: &mut Dst, msg: *const ()) {
            unsafe {
                let msg: Arc<Msg> = core::ptr::read_unaligned(msg as *const Arc<Msg>);

                // Ensure dispatch as &Msg. No failure to coerce
                ctx.dispatch(&*msg as &Msg);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Arc<Msg>, Arc::clone(&self.0));
            buf
        };
        Envelope {
            dispatcher: sync_dispatcher::<Dst, Msg>,
            buf,
        }
    }
}

impl<Msg: Send + Sync + 'static> EnvelopeBuilder<Msg> for ArcEnvelopeBuilder<Msg> {
    fn build_envelope<Dst: Pipeline>(&self) -> Envelope<Dst> {
        unsafe fn arc_dispatcher<Dst: Pipeline, Msg>(ctx: &mut Dst, msg: *const ()) {
            unsafe {
                let msg: Arc<Msg> = core::ptr::read_unaligned(msg as *const Arc<Msg>);

                // Ensure dispatch as Arc. No deref coercion
                ctx.dispatch(&msg as &Arc<_>);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Arc<Msg>, Arc::clone(&self.0));
            buf
        };
        Envelope {
            dispatcher: arc_dispatcher::<Dst, Msg>,
            buf,
        }
    }
}

impl<Msg: Copy + Send + 'static> EnvelopeBuilder<Msg> for CopyEnvelopeBuilder<Msg> {
    fn build_envelope<Dst: Pipeline>(&self) -> Envelope<Dst> {
        unsafe fn copy_dispatcher<Dst: Pipeline, Msg>(thread: &mut Dst, msg: *const ()) {
            unsafe {
                let msg: Msg = core::ptr::read_unaligned(msg as *const Msg);
                thread.dispatch(&msg);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Msg, self.0);
            buf
        };
        Envelope {
            dispatcher: copy_dispatcher::<Dst, Msg>,
            buf,
        }
    }
}

mod private {
    pub trait Sealed {}
}

impl<Msg: Send + Sync + 'static> private::Sealed for SyncEnvelope<'_, Msg> {}
impl<Msg: Send + Sync + 'static> private::Sealed for ArcEnvelope<'_, Msg> {}
impl<Msg: Copy + Send + 'static> private::Sealed for CopyEnvelope<'_, Msg> {}

pub trait ThreadLocalEnvelope<Msg>: private::Sealed {
    #[must_use]
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg>;
    fn get(&self) -> &Msg;
}

impl<Msg: Send + Sync + 'static> ThreadLocalEnvelope<Msg> for SyncEnvelope<'_, Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        // SAFETY: The referent is not dropped in `forward_as_envelope`
        SyncEnvelopeBuilder(Arc::new(unsafe { mem::ManuallyDrop::take(self.0) }))
    }

    fn get(&self) -> &Msg {
        self.0
    }
}

impl<Msg: Send + Sync + 'static> ThreadLocalEnvelope<Msg> for ArcEnvelope<'_, Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        // SAFETY: The referent is not dropped in `forward_as_envelope`
        ArcEnvelopeBuilder(unsafe { mem::ManuallyDrop::take(self.0) })
    }

    fn get(&self) -> &Msg {
        self.0
    }
}

impl<Msg: Copy + Send + 'static> ThreadLocalEnvelope<Msg> for CopyEnvelope<'_, Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        CopyEnvelopeBuilder(*self.0)
    }

    fn get(&self) -> &Msg {
        self.0
    }
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub struct __spec_envelope_maker<T>(PhantomData<T>);

impl<T> __spec_envelope_maker<T> {
    pub fn from_ref(_: &T) -> Self {
        Self(PhantomData)
    }
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait __forward_as_envelope_copy<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T);
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait __forward_as_envelope_sync<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T);
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait __forward_as_envelope_arc<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T);
}

// Copy implementation is most specialized, so try that first
impl<T: Copy + Send + Sync + 'static> __forward_as_envelope_copy<T> for &'_ &'_ __spec_envelope_maker<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T) {
        if const { mem::size_of::<T>() <= BUF_SIZE } {
            ctx.publish(CopyEnvelope(&val));
        } else {
            let mut val = mem::ManuallyDrop::new(val);
            ctx.publish(SyncEnvelope(&mut val));
        }
    }
}

// Then see if this is already an [`Arc`]
impl<T: Send + Sync + 'static> __forward_as_envelope_arc<Arc<T>> for &'_ __spec_envelope_maker<Arc<T>> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: Arc<T>) {
        let mut val = mem::ManuallyDrop::new(val);
        ctx.publish(ArcEnvelope(&mut val));
    }
}

// Then fall back to allocating at publish
impl<T: Send + Sync + 'static> __forward_as_envelope_sync<T> for __spec_envelope_maker<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T) {
        let mut val = mem::ManuallyDrop::new(val);
        ctx.publish(SyncEnvelope(&mut val));
    }
}

#[cfg(test)]
mod test;
