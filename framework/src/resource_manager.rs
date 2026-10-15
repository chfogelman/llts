use crate::{Hub, Pollable, ReplaySource};

pub enum ResourceManager<'a, H: Hub> {
    Live(&'a H::LiveResourceManager),
    Replay(&'a H::ReplayResourceManager),
}

pub trait LiveResourceManager: Hub {
    fn register_pollable(&self, pollable: Box<dyn Pollable<Self::Pipeline>>);
}

impl LiveResourceManager for ! {
    fn register_pollable(&self, _pollable: Box<dyn Pollable<Self::Pipeline>>) {
        unimplemented!()
    }
}

pub trait ReplayResourceManager: Hub {
    fn register_source(&self, source: Box<dyn ReplaySource<Self::Pipeline>>);
}

impl ReplayResourceManager for ! {
    fn register_source(&self, _source: Box<dyn ReplaySource<Self::Pipeline>>) {
        unimplemented!()
    }
}
