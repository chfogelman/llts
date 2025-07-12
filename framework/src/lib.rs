#![feature(min_specialization)]

pub trait Component {
    fn on_message<Msg>(&mut self, msg: &Msg);
}

impl<T> Component for T {
    #[inline]
    fn on_message<Msg>(&mut self, msg: &Msg) {
        <T as Handler<Msg>>::handle(self, msg);
    }
}

pub trait Handler<Msg> {
    fn handle(&mut self, msg: &Msg);
}

impl<Msg, T> Handler<Msg> for T {
    #[inline]
    default fn handle(&mut self, _: &Msg) {}
}

#[cfg(test)]
mod test {

    use super::*;

    struct ExampleComponent;
    struct Message;
    struct Message2;

    impl<Msg> Handler<Msg> for ExampleComponent {
        default fn handle(&mut self, _: &Msg) {
            panic!("Hello");
        }
    }

    impl Handler<Message> for ExampleComponent {
        fn handle(&mut self, _: &Message) {}
    }

    impl Handler<Message2> for ExampleComponent {
        fn handle(&mut self, _: &Message2) {}
    }

    #[test]
    fn test_receives() {
        let mut ex1 = ExampleComponent;
        ex1.on_message(&Message);
        ex1.on_message(&Message2);
        ex1.on_message(&mut Message);
    }

    #[test]
    #[should_panic]
    fn test_not_receives() {
        let mut ex = ExampleComponent;
        struct Foo;
        ex.on_message(&Foo);
    }
}
