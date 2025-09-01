#![feature(min_specialization)]
#![allow(dead_code)] // FIXME

mod thread_builder;
pub use thread_builder::ThreadBuilder;
pub use thread_builder::ThreadStart;

use thread_builder::ThreadInfo;

pub struct PollEvent;

pub trait Consumer {
    fn publish<Msg: Send + Sync + 'static>(&mut self, msg: Msg);
}

pub trait Handler<Msg> {
    fn handle(&mut self, ctx: &mut impl Consumer, msg: &Msg);
}

/// No-op default handler implementation for `T`
impl<Msg, T> Handler<Msg> for T {
    #[inline]
    default fn handle(&mut self, _ctx: &mut impl Consumer, _: &Msg) {}
}

impl<Msg, T> Handler<Msg> for &mut T {
    #[inline]
    fn handle(&mut self, ctx: &mut impl Consumer, msg: &Msg) {
        (**self).handle(ctx, msg);
    }
}

pub trait Dispatcher<T>
where Self: Send + Sync 
{
    fn dispatch(&self, thread: &mut T);
}

impl<T: ThreadInfo, Msg: Send + Sync + 'static> Dispatcher<T> for Msg
{
    fn dispatch(&self, thread: &mut T) {
        let (component, handles) = thread.get_handler_parts();
        <T::Component as Handler<Msg>>::handle(component, handles, self);
    }
}

/// No-op Consumer implementation for `()`
impl Consumer for () {
    fn publish<Msg>(&mut self, _msg: Msg) {}
}

pub struct ChainedComponent<T, U>(pub T, pub U);

impl<Msg, T, U> Handler<Msg> for ChainedComponent<T, U> {
    fn handle(&mut self, ctx: &mut impl Consumer, msg: &Msg) {
        let mut downstream = DownstreamComponents(&mut self.1, &mut *ctx);
        <T as Handler<Msg>>::handle(&mut self.0, &mut downstream, msg);
        <U as Handler<Msg>>::handle(&mut self.1, ctx, msg);
    }
}

struct DownstreamComponents<'a, T, U>(&'a mut T, &'a mut U);

impl<'a, T, U> Consumer for DownstreamComponents<'a, T, U>
where
    U: Consumer,
{
    fn publish<Msg: Send + Sync + 'static>(&mut self, msg: Msg) {
        <T as Handler<Msg>>::handle(self.0, self.1, &msg);
        self.1.publish(msg);
    }
}

#[macro_export]
macro_rules! chain_components_impl {
    ($first:expr) => { $first };
    ($last:expr, $($rest:expr),+) => { $crate::ChainedComponent($crate::chain_components_impl!($($rest),+), $last) };
}

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
        fn handle(&mut self, ctx: &mut impl Consumer, msg: &Message0) {
            println!(
                "{:width$}Ex1({}) received Message0({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(Message1("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message1> for Ex1 {
        fn handle(&mut self, ctx: &mut impl Consumer, msg: &Message1) {
            println!(
                "{:width$}Ex1({}) received Message1({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(Message0("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message0> for Ex2 {
        fn handle(&mut self, ctx: &mut impl Consumer, msg: &Message0) {
            println!(
                "{:width$}Ex2({}) received Message0({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(Message0("Ex2", self.0, msg.2 + 4));
        }
    }

    #[test]
    fn test_receives() {
        let mut ex1 = Ex1("Ex1");
        struct Tester(bool);
        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Consumer, _msg: &Message1) {
                self.0 = true;
            }
        }
        impl Consumer for Tester {
            fn publish<Msg>(&mut self, msg: Msg) {
                <Self as Handler<Msg>>::handle(self, &mut (), &msg);
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
            fn handle(&mut self, _ctx: &mut impl Consumer, msg: &Message0) {
                println!("Tester received Message0({})", msg.0);
                self.0 += 1;
            }
        }

        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Consumer, msg: &Message1) {
                println!("Tester received Message1({})", msg.0);
                self.1 += 1;
            }
        }

        impl Consumer for Tester {
            fn publish<Msg>(&mut self, msg: Msg) {
                <Self as Handler<Msg>>::handle(self, &mut (), &msg);
            }
        }

        let mut foo = chain_components!(Ex2("first"), Ex2("second"), Ex1("third"));
        let mut tester = Tester(0, 0);
        foo.handle(&mut tester, &Message0("init", "init", 0));
        assert_eq!(tester.0, 3);
        assert_eq!(tester.1, 4);
    }
}
