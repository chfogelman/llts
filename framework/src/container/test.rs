use super::*;
use crate::publish;

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

        fn stop(&mut self) {
            unimplemented!()
        }
    }

    let mut tester = Tester(false);
    ex1.handle(&mut tester, &Message0("init", "init", 0));
    assert!(tester.0);
}

#[test]
fn test_chain() {
    struct Tester<'a>(&'a mut i8, &'a mut i8);

    impl Handler<Message0> for Tester<'_> {
        fn handle(&mut self, _ctx: &mut impl Context, msg: &Message0) {
            println!("Tester received Message0({})", msg.0);
            *self.0 += 1;
        }
    }

    impl Handler<Message1> for Tester<'_> {
        fn handle(&mut self, _ctx: &mut impl Context, msg: &Message1) {
            println!("Tester received Message1({})", msg.0);
            *self.1 += 1;
        }
    }

    impl Context for Tester<'_> {
        fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
            <Self as Handler<Msg>>::handle(self, &mut (), msg.get());
        }

        fn stop(&mut self) {
            unimplemented!()
        }
    }

    let mut msg0_qty = 0;
    let mut msg1_qty = 0;

    let tester = Tester(&mut msg0_qty, &mut msg1_qty);
    let mut foo = PipelineBuilder
        .with(Ex2("first"))
        .with(Ex2("second"))
        .with(Ex1("third"))
        .with_context(tester);
    foo.dispatch(&Message0("init", "init", 0));
    assert_eq!(msg0_qty, 3);
    assert_eq!(msg1_qty, 4);
}

