#![feature(min_specialization)]
#![allow(dead_code)] // FIXME
#![allow(unused_imports)] // FIXME

use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};

pub trait Component {
    fn publish<Msg>(&mut self, msg: &Msg);
}

pub trait Handler<Msg> {
    fn handle(&mut self, ctx: &mut impl Component, msg: &Msg);
}

trait Dispatcher<C: Component> {
    fn dispatch(&self, component: &mut C);
}

impl<C, Msg> Dispatcher<C> for Msg {
    fn dispatch(&self, component: &mut C) {
        component.publish(self)
    }
}

impl<T> Component for T {
    #[inline]
    default fn publish<Msg>(&mut self, msg: &Msg) {
        <T as Handler<Msg>>::handle(self, &mut (), msg);
    }
}

impl Component for () {
    #[inline]
    fn publish<Msg>(&mut self, _: &Msg) {}
}

/// No-op default handler implementation for `T`
impl<Msg, T> Handler<Msg> for T {
    #[inline]
    default fn handle(&mut self, _ctx: &mut impl Component, _: &Msg) {}
}

/// No-op handler implementation for `()`
impl<Msg> Handler<Msg> for () {
    #[inline]
    fn handle(&mut self, _ctx: &mut impl Component, _: &Msg) {}
}

pub struct ChainedComponent<T, U>(T, U);

impl<T, U> Component for ChainedComponent<T, U> {
    fn publish<Msg>(&mut self, msg: &Msg) {
        self.0.handle(&mut self.1, msg);
        self.1.publish(msg);
    }
}

pub struct Thread<C, Others> {
    component: C,
    in_q: Receiver<Arc<dyn Dispatcher<C>>>,
    out_qs: Others,
}

impl<C, Others> Thread<C, Others> {
    fn new(component: C, in_q: Receiver<Arc<dyn Dispatcher<C>>>, out_qs: Others) -> Self {
        Thread {
            component,
            in_q,
            out_qs,
        }
    }
}

type PreThreadTriplet<C> = (C, Sender<Arc<dyn Dispatcher<C>>>, Receiver<Arc<dyn Dispatcher<C>>>);

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

impl_get_senders!{A, B, C, D, E, F, G, H, I, J,}

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
    ($($ts:ident,)*) => {}
}

fn foo() {
    let x = (get_channel(0i32), get_channel(1u32));
    let senders = x.get_senders();
    let x = (x, senders).transpose();
}

#[macro_export]
macro_rules! chain_components {
    ($first:expr) => { $first };
    ($first:expr, $($rest:expr),+) => { ChainedComponent($first, chain_components!($($rest),+)) };
}

#[cfg(test)]
mod test {

    use super::*;

    struct Ex1;
    struct Ex2;
    struct Message;
    struct Message2;

    impl<Msg> Handler<Msg> for Ex1 {
        default fn handle(&mut self, _ctx: &mut impl Component, _: &Msg) {
            panic!("Ex1 panicked");
        }
    }

    impl Handler<Message> for Ex1 {
        fn handle(&mut self, _ctx: &mut impl Component, _: &Message) {
            println!("Ex1");
        }
    }

    impl Handler<Message2> for Ex1 {
        fn handle(&mut self, _ctx: &mut impl Component, _: &Message2) {
            println!("Ex1");
        }
    }

    #[test]
    fn test_receives() {
        let mut ex1 = Ex1;
        ex1.publish(&Message);
        ex1.publish(&Message2);
        ex1.publish(&mut Message);
    }

    #[test]
    #[should_panic]
    fn test_not_receives() {
        let mut ex = Ex1;
        struct Foo;
        ex.publish(&Foo);
    }

    #[test]
    #[should_panic = "Ex1 panicked"]
    fn test_chain() {
        struct Msg;
        let mut foo = chain_components!(Ex2, Ex2, Ex1);
        foo.publish(&Msg);
    }
}
