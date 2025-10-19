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
mod test {

    use super::*;
    use crate::container::{Container, PipelineBuilder};

    struct Ex1(&'static str);
    struct Ex2(&'static str);
    struct Message0(&'static str, &'static str, usize);
    struct Message1(&'static str, &'static str, usize);

    impl Handler<Message0> for Ex1 {
        fn handle(&mut self, ctx: &mut impl Context, msg: &Message0) {
            println!("{:width$}Ex1({}) received Message0({}, {})", "", self.0, msg.0, msg.1, width = msg.2);
            publish!(ctx, Message1("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message1> for Ex1 {
        fn handle(&mut self, ctx: &mut impl Context, msg: &Message1) {
            println!("{:width$}Ex1({}) received Message1({}, {})", "", self.0, msg.0, msg.1, width = msg.2);
            publish!(ctx, Message0("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message0> for Ex2 {
        fn handle(&mut self, ctx: &mut impl Context, msg: &Message0) {
            println!("{:width$}Ex2({}) received Message0({}, {})", "", self.0, msg.0, msg.1, width = msg.2);
            publish!(ctx, Message0("Ex2", self.0, msg.2 + 4));
        }
    }

    #[test]
    fn test_receives() {
        let ex1 = Ex1("Ex1");
        struct Tester(bool);
        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Context, _msg: &Message1) {
                self.0 = true;
            }
        }
        impl Context for &mut Tester {
            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                <Self as Handler<Msg>>::handle(self, &mut (), msg.get());
            }

            fn stop(&mut self) {
                unimplemented!()
            }
        }

        let mut tester = Tester(false);
        let mut pipeline = PipelineBuilder.with(ex1).with_context(&mut tester);
        pipeline.dispatch(&Message0("init", "init", 0));
        assert!(tester.0);
    }

    #[test]
    fn test_chain() {
        struct Tester(i8, i8);

        impl Handler<Message0> for Tester {
            fn handle(&mut self, _ctx: &mut impl Context, msg: &Message0) {
                println!("Tester received Message0({})", msg.0);
                self.0 += 1;
            }
        }

        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Context, msg: &Message1) {
                println!("Tester received Message1({})", msg.0);
                self.1 += 1;
            }
        }

        impl Context for &mut Tester {
            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                <Self as Handler<Msg>>::handle(self, &mut (), msg.get());
            }

            fn stop(&mut self) {
                unimplemented!()
            }
        }

        let mut tester = Tester(0, 0);
        let mut foo = chain_components!(Ex2("first"), Ex2("second"), Ex1("third")).with_context(&mut tester);
        foo.dispatch(&Message0("init", "init", 0));
        assert_eq!(tester.0, 3);
        assert_eq!(tester.1, 4);
    }
}
