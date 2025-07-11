#![feature(min_specialization)]

struct Message;
struct Message2;

struct ExampleComponent;

trait Component {
    fn on_message<Msg>(&mut self, msg: &Msg);
}

impl Component for ExampleComponent {
    fn on_message<Msg>(&mut self, msg: &Msg) {
        trait Dispatcher<M> {
            fn dispatch(&self, r: &mut ExampleComponent);
        }

        impl<M> Dispatcher<M> for M {
            #[inline]
            default fn dispatch(&self, _r: &mut ExampleComponent) {
                panic!("Hello");
            }
        }

        impl<'a, M> Dispatcher<&'a M> for &'a M {
            #[inline]
            fn dispatch(&self, r: &mut ExampleComponent) {
                (**self).dispatch(r);
            }
        }

        impl<'a, M> Dispatcher<&'a mut M> for &'a mut M {
            #[inline]
            fn dispatch(&self, r: &mut ExampleComponent) {
                (**self).dispatch(r);
            }
        }

        impl Dispatcher<Message> for Message {
            fn dispatch(&self, r: &mut ExampleComponent) {}
        }

        impl Dispatcher<Message2> for Message2 {
            fn dispatch(&self, r: &mut ExampleComponent) {}
        }

        <Msg as Dispatcher<Msg>>::dispatch(msg, self);
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_receives() {

        fn foo<T>(t: &T) {
            ExampleComponent.on_message(&t);
        }

        foo(&Message);
        foo(&Message2);
    }

    #[test]
    #[should_panic]
    fn test_not_receives() {
        struct Message;
        ExampleComponent.on_message(&Message);
    }
}
