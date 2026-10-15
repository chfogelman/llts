use std::ops::ControlFlow;
use std::rc::Rc;

use crate::envelope::ThreadLocalEnvelope;
use crate::hub::Hub;
use crate::messages::*;
use crate::{Context, Pipeline};

pub struct PipelineFinalizer<H> {
    pub hub: Rc<H>,
}

impl<H> Context for PipelineFinalizer<H>
where
    H: Hub,
{
    type Hub = H;

    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
        self.hub.publish(msg);
    }

    fn get_hub(&self) -> &Self::Hub {
        &self.hub
    }
}

impl<H: Hub> Pipeline for PipelineFinalizer<H> {
    fn dispatch<Msg>(&mut self, _msg: &Msg) {
        // Intentionally blank; end of the line for dispatching
    }
}

pub trait Driver {
    type Pipeline: Pipeline;

    fn run(self) -> String;
}

//pub struct DriverImpl<P: Pipeline> {
//    hub:      Rc<P::Hub>,
//    pipeline: P
//}
//
//impl<C: Container, H: Hub> Driver<C::Finalized<PipelineFinalizer<H>>>
//{
//    pub fn new(p: C, hub: H) -> Self {
//        let hub = Rc::<H>::new(hub);
//        let pipeline = container.finalize(PipelineFinalizer { hub: Rc::clone(&hub) });
//        Self { hub, pipeline }
//    }
//}

pub(crate) fn run<P: Pipeline>(mut pipeline: P, hub: Rc<P::Hub>) -> String
where
    P::Hub: Hub<Pipeline = P>,
{
    pipeline.dispatch(&StartEvent);
    let reason = loop {
        let poll_result = hub.poll(&mut pipeline);
        match poll_result {
            ControlFlow::Continue(_) => {},
            ControlFlow::Break(reason) => {
                eprintln!("Stopping for reason {reason}");
                break reason;
            },
        }
    };
    pipeline.dispatch(&Shutdown);
    reason
}
