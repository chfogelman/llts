#![feature(min_specialization)]
#![allow(dead_code)] // FIXME
#![allow(unused_imports)] // FIXME

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

pub trait Component {
    fn publish<Msg>(&mut self, msg: &Msg);
}

pub trait Handler<Msg> {
    fn handle(&mut self, ctx: &mut impl Component, msg: &Msg);
}

trait Dispatcher<T> {
    fn dispatch(&self, thread: &mut T);
}

impl<C, Others, Msg> Dispatcher<Thread<C, Others>> for Msg
where
    Others: Component,
{
    fn dispatch(&self, thread: &mut Thread<C, Others>) {
        <C as Handler<Msg>>::handle(&mut thread.component, &mut thread.out_qs, self);
    }
}

/// No-op default handler implementation for `T`
impl<Msg, T> Handler<Msg> for T {
    #[inline]
    default fn handle(&mut self, _ctx: &mut impl Component, _: &Msg) {}
}

impl<Msg, T> Handler<Msg> for &mut T {
    #[inline]
    fn handle(&mut self, ctx: &mut impl Component, msg: &Msg) {
        (**self).handle(ctx, msg);
    }
}

/// No-op component implementation for `()`
impl Component for () {
    fn publish<Msg>(&mut self, _msg: &Msg) {}
}

pub struct ChainedComponent<T, U>(T, U);
struct DownstreamComponents<'a, T, U>(&'a mut T, &'a mut U);

impl<Msg, T, U> Handler<Msg> for ChainedComponent<T, U> {
    fn handle(&mut self, ctx: &mut impl Component, msg: &Msg) {
        let mut downstream = DownstreamComponents(&mut *ctx, &mut self.0);
        <U as Handler<Msg>>::handle(&mut self.1, &mut downstream, msg);
        <T as Handler<Msg>>::handle(&mut self.0, ctx, msg);
    }
}

impl<'a, T, U> Component for DownstreamComponents<'a, T, U>
where
    T: Component,
{
    fn publish<Msg>(&mut self, msg: &Msg) {
        <U as Handler<Msg>>::handle(self.1, self.0, msg);
        self.0.publish(msg);
    }
}

pub struct Thread<C, Others> {
    component: C,
    in_q: Receiver<Arc<dyn Dispatcher<Self>>>,
    out_qs: Others,
}

impl<C, Others: Component> Thread<C, Others> {
    fn new(component: C, in_q: Receiver<Arc<dyn Dispatcher<Self>>>, out_qs: Others) -> Self {
        Thread {
            component,
            in_q,
            out_qs,
        }
    }

    fn poll(&mut self) {
        while let Ok(msg) = self.in_q.recv() {
            msg.dispatch(self);
        }
    }
}

type PreThreadTriplet<C> = (
    C,
    Sender<Arc<dyn Dispatcher<C>>>,
    Receiver<Arc<dyn Dispatcher<C>>>,
);

fn get_channel<C>(component: C) -> PreThreadTriplet<C> {
    let (tx, rx) = channel();
    (component, tx, rx)
}

fn get_sender<C>(t: &PreThreadTriplet<C>) -> Sender<Arc<dyn Dispatcher<C>>> {
    t.1.clone()
}

trait Senders {
    type Output;

    fn get_senders(&self) -> Self::Output;
}

type ProducerHandle<C> = Sender<Arc<dyn Dispatcher<C>>>;

macro_rules! senders_output_helper {
    ( (), ($($prev:ident,)*), ($($accum:tt,)*) ) => {
        ($($accum,)*)
    };
    ( ($t:ident, $($ts:ident,)*), ($($prev:ident,)*), ($($accum:tt,)*) ) => {
        senders_output_helper!(($($ts,)*), ($($prev,)* $t,), ($($accum,)* ($(ProducerHandle<$prev>,)* $(ProducerHandle<$ts>,)*),))
    };
}

macro_rules! senders_impl_helper {
    ($self:ident, (), ($($prev:ident,)*), ($($accum:tt,)*)) => {
        ($($accum,)*)
    };
    ($self:ident, ($t:ident, $($ts:ident,)*), ($($prev:ident,)*), ($($accum:tt,)*) ) => {
        senders_impl_helper!($self, ($($ts,)*), ($($prev,)* $t,),
            ($($accum,)*
            ({
                let ($(ref $prev,)* ref $t, $(ref $ts,)*) = *$self;
                ($($prev.1.clone(),)* $($ts.1.clone(),)*)
            }),)
        )
    };
}

macro_rules! peel_get_senders {
    ($t:ident, $($ts:ident,)*) => {
        impl_get_senders!{$($ts,)*}
    };
}

macro_rules! impl_get_senders {
    () => {};
    ($($ts:ident,)+) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($ts),*> Senders for ($(PreThreadTriplet<$ts>,)*) {
            type Output = senders_output_helper!{ ($($ts,)*), (), () };

            fn get_senders(&self) -> Self::Output {
                senders_impl_helper!{self, ($($ts,)*), (), ()}
            }
        }
        peel_get_senders!{$($ts,)*}
    };
}

impl_get_senders! {A, B, C, D, E, F, G, H, I, J,}

trait Transpose2 {
    type Output;

    fn transpose(self) -> Self::Output;
}

macro_rules! peel_transpose2 {
    ((), ()) => {};
    (($t:ident, $($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl_transpose2!{ ($($ts,)*), ($($us,)*) }
    };
}

macro_rules! impl_transpose2 {
    (($($ts:ident,)*), ($($us:ident,)*)) => {
        #[allow(non_snake_case)]
        impl<$($ts,)*$($us,)*> Transpose2 for (($($ts,)*), ($($us,)*)) {
            type Output = ($($ts, $us),*);

            fn transpose(self) -> Self::Output {
                let (($($ts,)*), ($($us,)*)) = self;
                ($($ts, $us),*)
            }
        }
        peel_transpose2!{ ($($ts,)*), ($($us,)*) }
    };
}

impl_transpose2! {
    (A1, B1, C1, D1, E1, F1, G1, H1, I1, J1,),
    (A2, B2, C2, D2, E2, F2, G2, H2, I2, J2,)
}

macro_rules! impl_build_threads {
    ($($ts:ident,)*) => {};
}

#[macro_export]
macro_rules! chain_components {
    ($first:expr) => { $first };
    ($first:expr, $($rest:expr),+) => { ChainedComponent($first, chain_components!($($rest),+)) };
}

#[cfg(test)]
mod test {

    use super::*;

    struct Ex1(&'static str);
    struct Ex2(&'static str);
    struct Message0(&'static str, &'static str, usize);
    struct Message1(&'static str, &'static str, usize);

    impl Handler<Message0> for Ex1 {
        fn handle(&mut self, ctx: &mut impl Component, msg: &Message0) {
            println!(
                "{:width$}Ex1({}) received Message0({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(&Message1("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message1> for Ex1 {
        fn handle(&mut self, ctx: &mut impl Component, msg: &Message1) {
            println!(
                "{:width$}Ex1({}) received Message1({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(&Message0("Ex1", self.0, msg.2 + 4));
        }
    }

    impl Handler<Message0> for Ex2 {
        fn handle(&mut self, ctx: &mut impl Component, msg: &Message0) {
            println!(
                "{:width$}Ex2({}) received Message0({}, {})",
                "",
                self.0,
                msg.0,
                msg.1,
                width = msg.2
            );
            ctx.publish(&Message0("Ex2", self.0, msg.2 + 4));
        }
    }

    #[test]
    fn test_receives() {
        let mut ex1 = Ex1("Ex1");
        struct Tester(bool);
        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Component, _msg: &Message1) {
                self.0 = true;
            }
        }
        impl Component for Tester {
            fn publish<Msg>(&mut self, msg: &Msg) {
                <Tester as Handler<Msg>>::handle(self, &mut (), msg);
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
            fn handle(&mut self, _ctx: &mut impl Component, msg: &Message0) {
                println!("Tester received Message0({})", msg.0);
                self.0 += 1;
            }
        }

        impl Handler<Message1> for Tester {
            fn handle(&mut self, _ctx: &mut impl Component, msg: &Message1) {
                println!("Tester received Message1({})", msg.0);
                self.1 += 1;
            }
        }

        impl Component for Tester {
            fn publish<Msg>(&mut self, msg: &Msg) {
                <Self as Handler<Msg>>::handle(self, &mut (), msg);
            }
        }

        let mut foo = chain_components!(Ex1("first"), Ex2("second"), Ex2("third"));
        let mut tester = Tester(0, 0);
        foo.handle(&mut tester, &Message0("init", "init", 0));
        assert_eq!(tester.0, 3);
        assert_eq!(tester.1, 4);
    }
}
