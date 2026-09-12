use crate::envelope::ThreadLocalEnvelope;
use crate::{Context, Handler};

pub struct PipelineBuilder;

pub struct Layer<T, U>(T, U);

pub trait Container {
    type With<H>;

    fn with<H>(self, next: H) -> Self::With<H>;

    type WithContext<Ctx: Context>: Context;

    fn with_context<Ctx: Context>(self, ctx: Ctx) -> Self::WithContext<Ctx>;
}

impl Container for PipelineBuilder {
    type With<H> = Layer<H, Self>;

    fn with<H>(self, next: H) -> Self::With<H> {
        Layer(next, Self)
    }

    type WithContext<Ctx: Context> = Ctx;

    fn with_context<Ctx: Context>(self, ctx: Ctx) -> Self::WithContext<Ctx> {
        ctx
    }
}

impl<A, B> Container for Layer<A, B>
where
    B: Container,
{
    type With<H> = Layer<A, B::With<H>>;

    fn with<H>(self, next: H) -> Self::With<H> {
        Layer(self.0, self.1.with(next))
    }

    type WithContext<Ctx: Context> = Layer<A, B::WithContext<Ctx>>;

    fn with_context<Ctx: Context>(self, ctx: Ctx) -> Self::WithContext<Ctx> {
        Layer(self.0, self.1.with_context(ctx))
    }
}

impl<A, B> Context for Layer<A, B>
where
    B: Context,
{
    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
        <A as Handler<Msg>>::handle(&mut self.0, &mut self.1, msg.get());
        self.1.publish(msg);
    }

    fn dispatch<Msg>(&mut self, msg: &Msg) {
        <A as Handler<Msg>>::handle(&mut self.0, &mut self.1, msg);
        self.1.dispatch(msg)
    }

    #[inline]
    fn stop(&mut self) {
        self.1.stop();
    }
}

#[cfg(test)]
mod test;
