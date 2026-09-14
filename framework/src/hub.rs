use std::time::{Duration, SystemTime};

use crate::{Pipeline, Pollable};

pub mod live_hub;

pub trait Hub {
    type Pipeline: Pipeline;

    /// Poll the services owned by this `Hub`, such as networking, shared memory, and timers,
    /// and dispatch the results onto `pipeline`. This function should generally only be called
    /// by the framework.
    #[doc(hidden)]
    fn poll(&self, pipeline: &mut Self::Pipeline);

    fn register_pollable(&self, pollable: Box<dyn Pollable<Self::Pipeline>>);

    /// Enqueue a message to be dispatched into the owning pipeline on the next event loop.
    fn defer<Msg>(&self, msg: Msg);

    /// Get the current system time. This function should be preferred over direct use of
    /// [`SystemTime::now`], as this provides a replayed system time time in replay mode.
    fn now(&self) -> SystemTime;

    /// Start a timer which will expire after `duration`.
    /// The `event` argument carries a payload that will be looped back into the pipeline when the
    /// timer expires.
    fn start_timer<Event: Copy>(&self, event: Event, duration: Duration);

    /// Request graceful shutdown of the thread or process
    fn stop(&self, reason: String);
}
