use crate::Context;
use crate::thread_builder::ThreadInfo;
use std::sync::Arc;
use std::{marker::PhantomData, mem};

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
/// be incorrect for this type.
pub struct Envelope<Dst> {
    dispatcher: DispatchFn<Dst>,
    buf: EnvelopeBuf,
}

impl<Dst: ThreadInfo> Envelope<Dst> {
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

// Assert the size stays at 32 bytes
const _: () = const { assert!(mem::size_of::<Envelope<[u8; 1024]>>() == 32) };

// These are the [`ThreadLocalEnvelope`] types that are used to publish to handlers on the same
// thread. They are not converted to [`EnvelopeBuilder`]s until it is time to publish to other
// threads.
#[repr(transparent)]
pub(crate) struct CopyEnvelope<Msg: Copy + Send + 'static>(Msg);

impl<Msg: Copy + Send + 'static> CopyEnvelope<Msg> {
    /// Allows direct construction of [`CopyEnvelope`]s. This is used in this crate to send
    /// lifecycle messages between threads. This function checks for the proper size, unlike the
    /// `publish` macro which will not construct the wrong size.
    pub(crate) fn new(msg: Msg) -> Self {
        struct Check<T>(T);
        impl<T> Check<T> {
            const VALID: () = assert!(std::mem::size_of::<T>() <= BUF_SIZE);
        }
        let _ = Check::<Msg>::VALID;

        Self(msg)
    }
}

#[repr(transparent)]
struct SyncEnvelope<Msg: Send + Sync + 'static>(Msg);
#[repr(transparent)]
struct ArcEnvelope<Msg: Send + Sync + 'static>(Arc<Msg>);

// These are the [`EnvelopeBuilder`] types that may contain an allocated [`Arc`] that will be
// cloned when publishing to each other thread. These are only created after the contained message
// has been published to each thread-local [`Handler`].
#[repr(transparent)]
struct CopyEnvelopeBuilder<Msg>(Msg);
#[repr(transparent)]
struct SyncEnvelopeBuilder<Msg>(Arc<Msg>);
#[repr(transparent)]
struct ArcEnvelopeBuilder<Msg>(Arc<Msg>);

pub(crate) trait EnvelopeBuilder<Msg> {
    fn build_envelope<Dst: ThreadInfo>(&self) -> Envelope<Dst>;
}

impl<Msg: Copy + Send + 'static> EnvelopeBuilder<Msg> for CopyEnvelopeBuilder<Msg> {
    fn build_envelope<Thread: ThreadInfo>(&self) -> Envelope<Thread> {
        unsafe fn copy_dispatcher<T: ThreadInfo, Msg>(thread: &mut T, msg: *const ()) {
            unsafe {
                let msg: Msg = core::ptr::read_unaligned(msg as *const Msg);
                thread.dispatch_msg(&msg);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Msg, self.0);
            buf
        };
        Envelope {
            dispatcher: copy_dispatcher::<Thread, Msg>,
            buf,
        }
    }
}

impl<Msg: Send + Sync + 'static> EnvelopeBuilder<Msg> for SyncEnvelopeBuilder<Msg> {
    fn build_envelope<Thread: ThreadInfo>(&self) -> Envelope<Thread> {
        unsafe fn sync_dispatcher<T: ThreadInfo, Msg>(thread: &mut T, msg: *const ()) {
            unsafe {
                let msg: Arc<Msg> = core::ptr::read_unaligned(msg as *const Arc<Msg>);

                // Ensure dispatch as &Msg. No failure to coerce
                thread.dispatch_msg(&*msg as &Msg);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Arc<Msg>, Arc::clone(&self.0));
            buf
        };
        Envelope {
            dispatcher: sync_dispatcher::<Thread, Msg>,
            buf,
        }
    }
}

impl<Msg: Send + Sync + 'static> EnvelopeBuilder<Msg> for ArcEnvelopeBuilder<Msg> {
    fn build_envelope<Thread: ThreadInfo>(&self) -> Envelope<Thread> {
        unsafe fn arc_dispatcher<T: ThreadInfo, Msg>(thread: &mut T, msg: *const ()) {
            unsafe {
                let msg: Arc<Msg> = core::ptr::read_unaligned(msg as *const Arc<Msg>);

                // Ensure dispatch as Arc. No deref coercion
                thread.dispatch_msg(&msg as &Arc<_>);
            }
        }

        let buf = unsafe {
            let mut buf = [mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Arc<Msg>, Arc::clone(&self.0));
            buf
        };
        Envelope {
            dispatcher: arc_dispatcher::<Thread, Msg>,
            buf,
        }
    }
}

mod private {
    pub trait Sealed {}
}

impl<Msg: Copy + Send + 'static> private::Sealed for CopyEnvelope<Msg> {}
impl<Msg: Send + Sync + 'static> private::Sealed for SyncEnvelope<Msg> {}
impl<Msg: Send + Sync + 'static> private::Sealed for ArcEnvelope<Msg> {}

pub trait ThreadLocalEnvelope<Msg>: private::Sealed {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg>;
    fn get(&self) -> &Msg;
}

impl<Msg: Copy + Send + 'static> ThreadLocalEnvelope<Msg> for CopyEnvelope<Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        CopyEnvelopeBuilder(self.0)
    }

    fn get(&self) -> &Msg {
        &self.0
    }
}

impl<Msg: Send + Sync + 'static> ThreadLocalEnvelope<Msg> for SyncEnvelope<Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        SyncEnvelopeBuilder(Arc::new(self.0))
    }

    fn get(&self) -> &Msg {
        &self.0
    }
}

impl<Msg: Send + Sync + 'static> ThreadLocalEnvelope<Msg> for ArcEnvelope<Msg> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<Msg> {
        ArcEnvelopeBuilder(self.0)
    }

    fn get(&self) -> &Msg {
        &self.0
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
            ctx.publish(CopyEnvelope(val));
        } else {
            ctx.publish(SyncEnvelope(val));
        }
    }
}

// Then try Arc
impl<T: Send + Sync + 'static> __forward_as_envelope_arc<Arc<T>> for &'_ __spec_envelope_maker<Arc<T>> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: Arc<T>) {
        ctx.publish(ArcEnvelope(val));
    }
}

// Then fall back to allocating at publish
impl<T: Send + Sync + 'static> __forward_as_envelope_sync<T> for __spec_envelope_maker<T> {
    fn forward_as_envelope(&self, ctx: &mut impl Context, val: T) {
        ctx.publish(SyncEnvelope(val));
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::Handler;
    use crate::publish;

    fn helper<S, T>(s: &mut S, t: T) {
        <S as Handler<T>>::handle(s, &mut (), &t);
    }

    #[test]
    fn envelope_type_selection() {
        {
            struct Ctx;
            impl<T> Handler<T> for Ctx {
                default fn handle(&mut self, _: &mut impl Context, _: &T) {
                    panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
                }
            }

            impl Handler<CopyEnvelope<i32>> for Ctx {
                fn handle(&mut self, _: &mut impl Context, _: &CopyEnvelope<i32>) {
                    println!("Found correct handler.");
                }
            }

            impl Context for Ctx {
                fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                    helper(self, msg)
                }

                fn stop(&mut self) {
                    unimplemented!()
                }
            }
            publish!(&mut Ctx, 0i32);
        }
        {
            struct Ctx;
            impl<T> Handler<T> for Ctx {
                default fn handle(&mut self, _: &mut impl Context, _: &T) {
                    panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
                }
            }

            impl Handler<SyncEnvelope<[i32; 8]>> for Ctx {
                fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<[i32; 8]>) {
                    println!("Found correct handler.");
                }
            }

            impl Context for Ctx {
                fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                    msg.alloc_envelope();
                    //helper(self, msg)
                }

                fn stop(&mut self) {
                    unimplemented!()
                }
            }

            let val = [0i32; 8];
            publish!(&mut Ctx, val);
        }
        {
            struct Ctx;
            impl<T> Handler<T> for Ctx {
                default fn handle(&mut self, _: &mut impl Context, _: &T) {
                    panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
                }
            }

            impl Handler<SyncEnvelope<String>> for Ctx {
                fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<String>) {
                    println!("Found correct handler.");
                }
            }

            impl Context for Ctx {
                fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                    helper(self, msg)
                }

                fn stop(&mut self) {
                    unimplemented!()
                }
            }
            let val = String::new();
            publish!(&mut Ctx, val);
        }
        {
            struct NotCopy;

            struct Ctx;
            impl<T> Handler<T> for Ctx {
                default fn handle(&mut self, _: &mut impl Context, _: &T) {
                    panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
                }
            }

            impl Handler<SyncEnvelope<NotCopy>> for Ctx {
                fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<NotCopy>) {
                    println!("Found correct handler.");
                }
            }

            impl Context for Ctx {
                fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                    helper(self, msg)
                }

                fn stop(&mut self) {
                    unimplemented!()
                }
            }

            let val = NotCopy;
            publish!(&mut Ctx, val);
        }
        {
            struct Ctx;
            impl<T> Handler<T> for Ctx {
                default fn handle(&mut self, _: &mut impl Context, _: &T) {
                    panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
                }
            }

            impl Handler<ArcEnvelope<i32>> for Ctx {
                fn handle(&mut self, _: &mut impl Context, _: &ArcEnvelope<i32>) {
                    println!("Found correct handler.");
                }
            }

            impl Context for Ctx {
                fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                    helper(self, msg)
                }

                fn stop(&mut self) {
                    unimplemented!()
                }
            }

            let val = Arc::new(0i32);
            publish!(&mut Ctx, val)
        }
    }
}
