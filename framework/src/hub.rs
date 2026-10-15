use std::ops::ControlFlow;
use std::time::{Duration, SystemTime};

use crate::Pipeline;
use crate::envelope::ThreadLocalEnvelope;
use crate::resource_manager::{LiveResourceManager, ReplayResourceManager, ResourceManager};

pub mod live;
pub mod replay;

pub trait Hub: Sized {
    type Pipeline: Pipeline;

    type LiveResourceManager: LiveResourceManager;
    type ReplayResourceManager: ReplayResourceManager;

    /// Poll the services owned by this `Hub`, such as networking, shared memory, and timers,
    /// and dispatch the results onto `pipeline`. This function should generally only be called
    /// by the framework.
    #[doc(hidden)]
    fn poll(&self, pipeline: &mut Self::Pipeline) -> ControlFlow<String>;

    /// Publish the message across threads, as determined by the hub type and policy.
    ///
    /// By default, simply destroys the message, since by this point it's already been handled by
    /// everything on the current thread.
    #[doc(hidden)]
    fn publish<Msg>(&self, msg: impl ThreadLocalEnvelope<Msg>) {
        ThreadLocalEnvelope::drop(msg);
    }

    /// Enqueue a message to be dispatched into the owning pipeline on the next event loop.
    fn defer<Msg>(&self, msg: Msg);

    /// Get the current system time. This function should be preferred over direct use of
    /// [`SystemTime::now`], as this provides a replayed system time in replay mode.
    fn now(&self) -> SystemTime;

    /// Start a timer which will expire after `duration`.
    /// The `event` argument carries a payload that will be looped back into the pipeline when the
    /// timer expires.
    fn start_timer<Event: Copy>(&self, event: Event, duration: Duration);

    /// Request graceful shutdown of the thread or process
    fn stop(&self, reason: String);

    fn as_resource_manager(&self) -> ResourceManager<'_, Self>;
}

impl Hub for ! {
    type Pipeline = !;

    type LiveResourceManager = !;
    type ReplayResourceManager = !;

    fn poll(&self, _pipeline: &mut Self::Pipeline) -> ControlFlow<String> {
        unimplemented!()
    }

    fn defer<Msg>(&self, _msg: Msg) {
        unimplemented!()
    }

    fn now(&self) -> SystemTime {
        unimplemented!()
    }

    fn start_timer<Event: Copy>(&self, _event: Event, _duration: Duration) {
        unimplemented!()
    }

    fn stop(&self, _reason: String) {
        unimplemented!()
    }

    fn as_resource_manager(&self) -> ResourceManager<'_, Self> {
        unimplemented!()
    }
}
