use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use crate::container::Container;
use crate::driver::{Driver, PipelineFinalizer};
use crate::hub::ReplayResourceManager;
use crate::timer::TimerManager;
use crate::{Hub, ReplaySource, ResourceManager};

struct TimedSource<P> {
    next_time: SystemTime,
    source:    Box<dyn ReplaySource<P>>,
}

impl<P> PartialEq for TimedSource<P> {
    fn eq(&self, other: &Self) -> bool {
        other.next_time.eq(&self.next_time)
    }
}

impl<P> Eq for TimedSource<P> {}

impl<P> PartialOrd for TimedSource<P> {
    #![allow(clippy::non_canonical_partial_ord_impl)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.next_time.partial_cmp(&self.next_time)
    }
}

impl<P> Ord for TimedSource<P> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.next_time.cmp(&self.next_time)
    }
}

pub struct ReplayHub<D: Driver> {
    stop_requested: Cell<Option<String>>,
    clock_now:      Cell<SystemTime>,
    timer_manager:  TimerManager<D::Pipeline>,
    replay_sources: RefCell<BinaryHeap<TimedSource<D::Pipeline>>>,
}

impl<D: Driver> Default for ReplayHub<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Driver> ReplayHub<D> {
    pub fn new() -> Self {
        Self {
            stop_requested: Cell::new(None),
            clock_now:      Cell::new(SystemTime::UNIX_EPOCH),
            timer_manager:  TimerManager::new(2048),
            replay_sources: RefCell::new(BinaryHeap::new()),
        }
    }
}

pub fn make_driver<C: Container>(container: C) -> impl Driver {
    struct DriverImpl<C: Container> {
        hub:      Rc<ReplayHub<Self>>,
        pipeline: <Self as Driver>::Pipeline,
    }

    impl<C: Container> Driver for DriverImpl<C> {
        type Pipeline = C::Finalized<PipelineFinalizer<ReplayHub<Self>>>;

        fn run(self) -> String {
            crate::driver::run(self.pipeline, self.hub)
        }
    }
    let hub = Rc::new(ReplayHub::new());
    let pipeline = container.finalize(PipelineFinalizer { hub: Rc::clone(&hub) });
    DriverImpl::<C> { hub, pipeline }
}

impl<D: Driver> Hub for ReplayHub<D> {
    type Pipeline = D::Pipeline;

    type LiveResourceManager = !;
    type ReplayResourceManager = Self;

    fn poll(&self, pipeline: &mut Self::Pipeline) -> ControlFlow<String> {
        // TODO: the timer manager should be a `Pollable` and polled along with the rest of the
        // pollables
        self.timer_manager.poll(pipeline);
        // peek_mut allows us to modify the top element in-place
        if let Some(mut next_src) = self.replay_sources.borrow_mut().peek_mut() {
            self.clock_now.set(next_src.next_time);
            match next_src.source.poll(pipeline) {
                Ok(time) => {
                    next_src.next_time = time;
                    drop(next_src); // sift-down immediately
                },
                Err(_) => {
                    PeekMut::pop(next_src);
                },
            }
        } else {
            self.stop("Replay complete".into())
        }
        if let Some(reason) = self.stop_requested.take() {
            ControlFlow::Break(reason)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn defer<Msg>(&self, _msg: Msg) {
        todo!()
    }

    fn now(&self) -> SystemTime {
        self.clock_now.get()
    }

    fn start_timer<Event: Copy>(&self, event: Event, duration: Duration) {
        self.timer_manager.start_timer(event, self.now() + duration);
    }

    fn stop(&self, reason: String) {
        self.stop_requested.set(Some(reason));
    }

    fn as_resource_manager(&self) -> ResourceManager<'_, Self> {
        ResourceManager::Replay(self)
    }
}

impl<D: Driver> ReplayResourceManager for ReplayHub<D> {
    fn register_source(&self, source: Box<dyn ReplaySource<Self::Pipeline>>) {
        let start = source.advance_to(self.now()).expect("advance_to to be successful");
        self.replay_sources.borrow_mut().push(TimedSource {
            next_time: start,
            source,
        });
    }
}
