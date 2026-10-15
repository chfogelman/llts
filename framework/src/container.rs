use crate::envelope::ThreadLocalEnvelope;
use crate::{Context, Handler, Pipeline};

pub struct PipelineBuilder;

pub struct Layer<T, U>(T, U);

pub trait Container {
    type With<H>;

    fn with<H>(self, next: H) -> Self::With<H>;

    type Finalized<P: Pipeline>: Pipeline<Hub = P::Hub>;

    fn finalize<P: Pipeline>(self, pipeline: P) -> Self::Finalized<P>;
}

impl Container for PipelineBuilder {
    type With<H> = Layer<H, Self>;

    fn with<H>(self, next: H) -> Self::With<H> {
        Layer(next, Self)
    }

    type Finalized<P: Pipeline> = P;

    fn finalize<P: Pipeline>(self, pipeline: P) -> Self::Finalized<P> {
        pipeline
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

    type Finalized<P: Pipeline> = Layer<A, B::Finalized<P>>;

    fn finalize<P: Pipeline>(self, pipeline: P) -> Self::Finalized<P> {
        Layer(self.0, self.1.finalize(pipeline))
    }
}

impl<A, B> Context for Layer<A, B>
where
    B: Context,
{
    type Hub = B::Hub;

    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
        <A as Handler<Msg>>::handle(&mut self.0, &mut self.1, msg.get());
        self.1.publish(msg);
    }

    #[inline]
    fn get_hub(&self) -> &Self::Hub {
        self.1.get_hub()
    }
}

impl<A, B> Pipeline for Layer<A, B>
where
    B: Pipeline,
{
    fn dispatch<Msg>(&mut self, msg: &Msg) {
        <A as Handler<Msg>>::handle(&mut self.0, &mut self.1, msg);
        self.1.dispatch(msg)
    }
}

#[cfg(test)]
mod test;
