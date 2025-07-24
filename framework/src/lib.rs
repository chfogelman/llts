#![feature(min_specialization)]

pub trait Component {
    fn publish<Msg>(&mut self, msg: &Msg);
}

pub trait Handler<Msg> {
    fn handle(&mut self, ctx: &mut impl Component, msg: &Msg);
}

mod impls {
    impl<T> super::Component for T {
        #[inline]
        default fn publish<Msg>(&mut self, msg: &Msg) {
            <T as super::Handler<Msg>>::handle(self, &mut (), msg);
        }
    }

    impl super::Component for () {
        #[inline]
        fn publish<Msg>(&mut self, _: &Msg) {}
    }

    impl<Msg, T> super::Handler<Msg> for T {
        #[inline]
        default fn handle(&mut self, _ctx: &mut impl super::Component, _: &Msg) {}
    }

    impl<Msg> super::Handler<Msg> for () {
        #[inline]
        fn handle(&mut self, _ctx: &mut impl super::Component, _: &Msg) {}
    }
}

pub struct ChainedComponent<T, U>(T, U);

impl<T, U> Component for ChainedComponent<T, U> {
    fn publish<Msg>(&mut self, msg: &Msg) {
        self.0.handle(&mut self.1, msg);
        self.1.publish(msg);
    }
}

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
