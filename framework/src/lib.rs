#![feature(min_specialization)]

struct Message;

struct Component;

impl Component {
    pub fn do_something(&mut self, msg: &Message) {
        panic!("Hello");
    }
}

impl Component {
    pub fn on_message<Msg>(&mut self, msg: &Msg) {
        trait Dispatcher<M> {
            fn dispatch(&self, r: &mut Component);
        }

        impl<M> Dispatcher<M> for M {
            default fn dispatch(&self, _r: &mut Component) {}
        }

        impl<'a, M> Dispatcher<&'a M> for &'a M {
            fn dispatch(&self, r: &mut Component) {
                (**self).dispatch(r);
            }
        }

        impl<'a, M> Dispatcher<&'a mut M> for &'a mut M {
            fn dispatch(&self, r: &mut Component) {
                (**self).dispatch(r);
            }
        }

        impl Dispatcher<Message> for Message {
            fn dispatch(&self, r: &mut Component) {
                r.do_something(self);
            }
        }

        <Msg as Dispatcher<Msg>>::dispatch(msg, self);
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    #[should_panic]
    fn test_receives() {

        fn foo<T>(t: &T) {
            Component.on_message(&t);
        }

        foo(&Message);
    }

    #[test]
    fn test_not_receives() {

    }
}
