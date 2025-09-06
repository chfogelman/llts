use crate::Context;
use crate::thread_builder::ThreadInfo;
use std::sync::Arc;
use std::{marker::PhantomData, mem};

// Type-erased function for dispatching the contents of an envelope onto a thread
// managing a component chain
type DispatchFn<T> = unsafe fn(&mut T, *const ());

// Byte buffer for storing a type-erased object inside an envelope
type EnvelopeBuf = [mem::MaybeUninit<u8>; 24];

/// Type-erased envelope for sending messages between threads.
///
/// It provides optimizations for
/// small [`Copy`] types as well as [`Arc`]. You should not need to use this type
/// directly. Instead, use [`crate::publish`]. Because it may contain a type-erased [`Arc`], it
/// must _not_ implement [`Copy`] itself. The derived Clone implementation would also be incorrect for
/// this type.
pub struct Envelope<Dst> {
    dispatcher: DispatchFn<Dst>,
    buf: EnvelopeBuf,
}

impl<Dst: ThreadInfo> Envelope<Dst> {
    pub(crate) fn dispatch_onto(self, thread: &mut Dst) {
        let Envelope { dispatcher, ref buf } = self;
        unsafe { dispatcher(thread, buf.as_ptr() as *const ()) }
    }
}

// Assert the size stays at 32 bytes
const _: () = const { assert!(mem::size_of::<Envelope<[u8; 1024]>>() == 32) };

#[repr(transparent)]
struct CopyEnvelope<Msg: Copy + Send + 'static>(Msg);

#[repr(transparent)]
struct SyncEnvelope<Msg: Send + Sync + 'static>(Msg);

#[repr(transparent)]
struct ArcEnvelope<Msg: Send + Sync + 'static>(Arc<Msg>);

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

                // Ensure dispatch as &Msg
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

                // Ensure dispatch as Arc
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

pub trait ThreadLocalEnvelope<T>: private::Sealed {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<T>;
    fn get(&self) -> &T;
}

impl<T: Copy + Send + 'static> ThreadLocalEnvelope<T> for CopyEnvelope<T> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<T> {
        CopyEnvelopeBuilder(self.0)
    }

    fn get(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> ThreadLocalEnvelope<T> for SyncEnvelope<T> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<T> {
        SyncEnvelopeBuilder(Arc::new(self.0))
    }

    fn get(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> ThreadLocalEnvelope<T> for ArcEnvelope<T> {
    fn alloc_envelope(self) -> impl EnvelopeBuilder<T> {
        ArcEnvelopeBuilder(self.0)
    }

    fn get(&self) -> &T {
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
        if const { mem::size_of::<T>() <= mem::size_of::<EnvelopeBuf>() } {
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
                    helper(self, msg)
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
            }

            let val = Arc::new(0i32);
            publish!(&mut Ctx, val)
        }
    }
}
