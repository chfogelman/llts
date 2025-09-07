#![feature(min_specialization)]

pub mod envelope;
pub mod messages;
mod thread_builder;

pub use messages::*;
pub use thread_builder::ThreadBuilder;
pub use thread_builder::ThreadStart;

use envelope::ThreadLocalEnvelope;

pub trait Context {
    #[doc(hidden)]
    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>);
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
}

pub struct ChainedComponent<T, U>(pub T, pub U);

impl<Msg, T, U> Handler<Msg> for ChainedComponent<T, U> {
    fn handle(&mut self, ctx: &mut impl Context, msg: &Msg) {
        let mut downstream = DownstreamComponents {
            handler_ref: &mut self.1,
            ctx_ref: &mut *ctx,
        };
        <T as Handler<Msg>>::handle(&mut self.0, &mut downstream, msg);
        <U as Handler<Msg>>::handle(&mut self.1, ctx, msg);
    }
}

struct DownstreamComponents<'a, H, Ctx> {
    handler_ref: &'a mut H,
    ctx_ref: &'a mut Ctx,
}

impl<'a, H, Ctx> Context for DownstreamComponents<'a, H, Ctx>
where
    Ctx: Context,
{
    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
        <H as Handler<Msg>>::handle(self.handler_ref, self.ctx_ref, msg.get());
        self.ctx_ref.publish(msg);
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! chain_components_impl {
    ($first:expr) => { $first };
    ($last:expr, $($rest:expr),+) => { $crate::ChainedComponent($crate::chain_components_impl!($($rest),+), $last) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! reverse_to_chained {
    ((), ($($all:expr,)+)) => { $crate::chain_components_impl!($($all),+) };
    (($first:expr, $($rest:expr,)*), ($($all:expr,)*)) => {
        $crate::reverse_to_chained!(($($rest,)*), ($first, $($all,)*))
    };
}

#[macro_export]
macro_rules! chain_components {
    ($($components:expr),+) => { $crate::reverse_to_chained!(($($components,)+), ()) }
}

#[cfg(test)]
mod test {

    use super::*;

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
        let mut ex1 = Ex1("Ex1");
        struct Tester(bool);
        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Context, _msg: &Message1) {
                self.0 = true;
            }
        }
        impl Context for Tester {
            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                <Self as Handler<Msg>>::handle(self, &mut (), msg.get());
            }
        }

        let mut tester = Tester(false);
        ex1.handle(&mut tester, &Message0("init", "init", 0));
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

        impl Context for Tester {
            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                <Self as Handler<Msg>>::handle(self, &mut (), msg.get());
            }
        }

        let mut foo = chain_components!(Ex2("first"), Ex2("second"), Ex1("third"));
        let mut tester = Tester(0, 0);
        foo.handle(&mut tester, &Message0("init", "init", 0));
        assert_eq!(tester.0, 3);
        assert_eq!(tester.1, 4);
    }
}
