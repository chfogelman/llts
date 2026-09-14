use std::cmp::{Ord, PartialOrd};
use std::mem;
use std::time::SystemTime;

use util::unmut::binary_heap::BinaryHeap;

use crate::{Hub, Pipeline};

const PAYLOAD_SIZE: usize = 16;

struct Timer<P> {
    expiration: SystemTime, // TODO: replace with 6-byte time
    dispatcher: unsafe fn(&mut P, *const ()),
    payload:    [mem::MaybeUninit<u8>; PAYLOAD_SIZE],
}

struct TimerQueueElement<P>(Timer<P>);

impl<P> PartialEq for TimerQueueElement<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0.expiration == other.0.expiration
    }
}

impl<P> Eq for TimerQueueElement<P> {}

impl<P> PartialOrd for TimerQueueElement<P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Reversed because BinaryHeap is a max heap
        other.0.expiration.partial_cmp(&self.0.expiration)
    }
}

impl<P> Ord for TimerQueueElement<P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed because BinaryHeap is a max heap
        other.0.expiration.cmp(&self.0.expiration)
    }
}

pub struct TimerManager<P> {
    timers: BinaryHeap<TimerQueueElement<P>>,
}

impl<P> TimerManager<P> {
    pub fn new(capacity: usize) -> Self {
        Self {
            timers: BinaryHeap::with_capacity(capacity),
        }
    }
}

impl<P: Pipeline> TimerManager<P> {
    pub fn start_timer<Payload: Copy>(&self, payload: Payload, expiration: SystemTime) {
        util::const_assert_sizeof!(Payload, <=, PAYLOAD_SIZE, "Timer payload must be at most 16 bytes");

        unsafe fn timer_dispatcher<P: Pipeline, Payload>(pipeline: &mut P, payload: *const ()) {
            let payload = unsafe { core::ptr::read_unaligned(payload as *const Payload) };
            pipeline.dispatch(&payload);
        }

        let buf = unsafe {
            let mut buf = [std::mem::MaybeUninit::uninit(); _];
            core::ptr::write_unaligned(buf.as_mut_ptr() as *mut Payload, payload);
            buf
        };
        let timer = Timer {
            expiration,
            dispatcher: timer_dispatcher::<P, Payload>,
            payload: buf,
        };
        self.timers.push(TimerQueueElement(timer));
    }

    pub fn poll(&self, pipeline: &mut P) {
        let now = pipeline.get_hub().now();
        unsafe {
            // SAFETY: The predicate only inspects the first element, not modifying the binary heap
            // binary heap
            while let Some(TimerQueueElement(timer)) = self.timers.pop_if(|tqe| tqe.0.expiration <= now) {
                Self::dispatch_timer(pipeline, timer);
            }
        }
    }

    fn dispatch_timer(pipeline: &mut P, timer: Timer<P>) {
        let Timer {
            dispatcher,
            ref payload,
            ..
        } = timer;
        unsafe { dispatcher(pipeline, payload.as_ptr() as *const ()) }
    }
}

#[cfg(test)]
mod test;
