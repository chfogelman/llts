#![feature(min_specialization)]

pub mod container;
pub mod envelope;
pub mod messages;
mod thread_builder;

use envelope::ThreadLocalEnvelope;
pub use messages::*;
pub use thread_builder::ThreadBuilder;

pub trait Context {
    #[doc(hidden)]
    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>);

    #[allow(unused)]
    fn dispatch<Msg>(&mut self, msg: &Msg) {}

    fn stop(&mut self);
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

/// No-op [`Context`] implementation for `()`
impl Context for () {
    fn publish<Msg>(&mut self, _msg: impl ThreadLocalEnvelope<Msg>) {}

    fn stop(&mut self) {}
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
