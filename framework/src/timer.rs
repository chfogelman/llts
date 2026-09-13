use std::cmp::{Ord, PartialOrd};
use std::mem;
use std::time::{Duration, SystemTime};

use util::unmut::binary_heap::BinaryHeap;

use crate::Context;

const PAYLOAD_SIZE: usize = 16;

struct Timer<Ctx> {
    expiration: SystemTime, // TODO: replace with 6-byte time
    dispatcher: unsafe fn(&mut Ctx, *const ()),
    payload:    [mem::MaybeUninit<u8>; PAYLOAD_SIZE],
}

struct TimerQueueElement<Ctx>(Timer<Ctx>);

impl<Ctx> PartialEq for TimerQueueElement<Ctx> {
    fn eq(&self, other: &Self) -> bool {
        self.0.expiration == other.0.expiration
    }
}

impl<Ctx> Eq for TimerQueueElement<Ctx> {}

impl<Ctx> PartialOrd for TimerQueueElement<Ctx> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Reversed because BinaryHeap is a max heap
        other.0.expiration.partial_cmp(&self.0.expiration)
    }
}

impl<Ctx> Ord for TimerQueueElement<Ctx> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed because BinaryHeap is a max heap
        other.0.expiration.cmp(&self.0.expiration)
    }
}

pub struct TimerManager<Ctx> {
    timers: BinaryHeap<TimerQueueElement<Ctx>>,
}

impl<Ctx> TimerManager<Ctx> {
    pub fn new(capacity: usize) -> Self {
        Self {
            timers: BinaryHeap::with_capacity(capacity),
        }
    }
}

impl<Ctx: Context> TimerManager<Ctx> {
    pub fn start_timer<Payload: Copy>(&self, ctx: &Ctx, payload: Payload, duration: Duration) {
        util::const_assert_sizeof!(Payload, <=, PAYLOAD_SIZE, "Timer payload must be at most 16 bytes");

        unsafe fn timer_dispatcher<Ctx: Context, Payload>(ctx: &mut Ctx, payload: *const ()) {
            let payload = unsafe { core::ptr::read_unaligned(payload as *const Payload) };
            ctx.dispatch(&payload);
        }

        let buf = unsafe {
            let mut buf = [std::mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Payload, payload);
            buf
        };
        let timer = Timer {
            expiration: ctx.now() + duration,
            dispatcher: timer_dispatcher::<Ctx, Payload>,
            payload:    buf,
        };
        self.timers.push(TimerQueueElement(timer));
    }

    pub fn poll(&self, ctx: &mut Ctx) {
        let now = ctx.now();
        while let Some(TimerQueueElement(timer)) = self.timers.pop_if(|tqe| tqe.0.expiration <= now) {
            Self::dispatch_timer(ctx, timer);
        }
    }

    fn dispatch_timer(ctx: &mut Ctx, timer: Timer<Ctx>) {
        let Timer {
            dispatcher,
            ref payload,
            ..
        } = timer;
        unsafe { dispatcher(ctx, payload.as_ptr() as *const ()) }
    }
}

#[cfg(test)]
mod test;
