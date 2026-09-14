#![feature(min_specialization)]

pub mod container;
pub mod envelope;
mod hub;
pub mod messages;
mod thread_builder;
mod timer;

use std::time::{Duration, SystemTime};

use envelope::ThreadLocalEnvelope;
pub use hub::Hub;
pub use messages::*;
pub use thread_builder::ThreadBuilder;

pub trait Context {
    type Hub: Hub;

    #[doc(hidden)]
    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>);

    /// Get the [`Hub`] for this pipeline
    fn get_hub(&self) -> &Self::Hub;
}

pub trait Pipeline: Context {
    /// Dispatch an input message onto this pipeline
    fn dispatch<Msg>(&mut self, msg: &Msg);
}

#[derive(Debug, Clone, Copy)]
pub struct PollError; // TODO

pub trait Pollable<Target: Pipeline> {
    fn poll(&self, target: &mut Target) -> Result<(), PollError>;
}

pub struct PollableFn<F>(pub F);

impl<T, P: Pipeline, F: Fn() -> Result<T, PollError>> Pollable<P> for PollableFn<F> {
    fn poll(&self, target: &mut P) -> Result<(), PollError> {
        let v = self.0()?;
        let _: () = target.dispatch(&v);
        Ok(())
    }
}

#[macro_export]
macro_rules! publish {
    ($ctx:expr, $e:expr) => {{
        #[allow(unused_imports)]
        use $crate::envelope::{__forward_as_envelope_arc, __forward_as_envelope_copy, __forward_as_envelope_sync};
        let val = $e;
        let disp = $crate::envelope::__spec_envelope_maker::from_ref(&val);
        (&&disp).forward_as_envelope(&mut *$ctx, val)
    }};
}

pub trait Handler<Msg> {
    fn handle(&mut self, ctx: &mut impl Context, msg: &Msg);
}

/// No-op default handler implementation for `T`
impl<Msg, T> Handler<Msg> for T {
    #[inline]
    default fn handle(&mut self, _ctx: &mut impl Context, _: &Msg) {}
}

impl<Msg, T> Handler<Msg> for &mut T {
    #[inline]
    default fn handle(&mut self, ctx: &mut impl Context, msg: &Msg) {
        <T as Handler<Msg>>::handle(*self, ctx, msg);
    }
}

/// No-op [`Pipeline`] implementation for `()`
impl Context for () {
    type Hub = ();

    fn publish<Msg>(&mut self, _msg: impl ThreadLocalEnvelope<Msg>) {}

    fn get_hub(&self) -> &Self::Hub {
        todo!()
    }
}

impl Pipeline for () {
    fn dispatch<Msg>(&mut self, _msg: &Msg) {
        todo!()
    }
}

impl Hub for () {
    type Pipeline = ();

    fn poll(&self, _pipeline: &mut Self::Pipeline) {
        unimplemented!()
    }

    fn register_pollable(&self, _pollable: Box<dyn Pollable<Self::Pipeline>>) {
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
}

#[macro_export]
macro_rules! chain_components {
    ($($components:expr),*) => {{
        use $crate::container::Container;
        $crate::container::PipelineBuilder$(.with($components))*
    }}
}

#[cfg(test)]
mod test;
