use super::*;
use crate::{Handler, publish};

fn helper<S, T>(s: &mut S, t: impl ThreadLocalEnvelope<T>) {
    s.handle(&mut (), &t);
    // alloc_envelope ensures the value is dropped so that the tests do not fail miri
    let _ = t.alloc_envelope();
}

#[test]
fn envelope_type_selection() {
    // WARNING: This test contains nonstandard usage of [`Handler`]. We are simply ensuring that
    // the correct envelope type is selected and published, and the [`Handler`] trait already
    // has the signature we need. You should not implement `Handler` for envelope types.
    {
        struct Ctx;
        impl<T> Handler<T> for Ctx {
            default fn handle(&mut self, _: &mut impl Context, _: &T) {
                panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
            }
        }

        impl Handler<CopyEnvelope<'_, i32>> for Ctx {
            fn handle(&mut self, _: &mut impl Context, _: &CopyEnvelope<i32>) {
                println!("Found correct handler.");
            }
        }

        impl Context for Ctx {
            type Hub = ();

            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                helper(self, msg)
            }

            fn get_hub(&self) -> &Self::Hub {
                unimplemented!()
            }
        }
        publish!(&mut Ctx, 0i32);
    }
    {
        struct Ctx;
        impl<T> Handler<T> for Ctx {
            default fn handle(&mut self, _: &mut impl Context, _: &T) {
                panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
            }
        }

        impl Handler<SyncEnvelope<'_, [i32; 8]>> for Ctx {
            fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<[i32; 8]>) {
                println!("Found correct handler.");
            }
        }

        impl Context for Ctx {
            type Hub = ();

            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                helper(self, msg)
            }

            fn get_hub(&self) -> &Self::Hub {
                unimplemented!()
            }
        }

        let val = [0i32; 8];
        publish!(&mut Ctx, val);
    }
    {
        struct Ctx;
        impl<T> Handler<T> for Ctx {
            default fn handle(&mut self, _: &mut impl Context, _: &T) {
                panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
            }
        }

        impl Handler<SyncEnvelope<'_, String>> for Ctx {
            fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<String>) {
                println!("Found correct handler.");
            }
        }

        impl Context for Ctx {
            type Hub = ();

            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                helper(self, msg)
            }

            fn get_hub(&self) -> &Self::Hub {
                unimplemented!()
            }
        }
        let val = String::new();
        publish!(&mut Ctx, val);
    }
    {
        struct NotCopy;

        struct Ctx;
        impl<T> Handler<T> for Ctx {
            default fn handle(&mut self, _: &mut impl Context, _: &T) {
                panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
            }
        }

        impl Handler<SyncEnvelope<'_, NotCopy>> for Ctx {
            fn handle(&mut self, _: &mut impl Context, _: &SyncEnvelope<NotCopy>) {
                println!("Found correct handler.");
            }
        }

        impl Context for Ctx {
            type Hub = ();

            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                helper(self, msg)
            }

            fn get_hub(&self) -> &Self::Hub {
                unimplemented!()
            }
        }

        let val = NotCopy;
        publish!(&mut Ctx, val);
    }
    {
        struct Ctx;
        impl<T> Handler<T> for Ctx {
            default fn handle(&mut self, _: &mut impl Context, _: &T) {
                panic!("Incorrect envelope type found: {}", std::any::type_name::<T>());
            }
        }

        impl Handler<ArcEnvelope<'_, i32>> for Ctx {
            fn handle(&mut self, _: &mut impl Context, _: &ArcEnvelope<i32>) {
                println!("Found correct handler.");
            }
        }

        impl Context for Ctx {
            type Hub = ();

            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                helper(self, msg)
            }

            fn get_hub(&self) -> &Self::Hub {
                unimplemented!()
            }
        }

        let val = Arc::new(0i32);
        publish!(&mut Ctx, val)
    }
}
