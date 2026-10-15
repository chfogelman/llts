use std::cell::{Cell, RefCell};
use std::ops::ControlFlow;
use std::time::SystemTime;

use crate::hub::LiveResourceManager;
use crate::timer::TimerManager;
use crate::{Hub, Pipeline, Pollable, ResourceManager};

pub struct LiveHub<P> {
    stop_requested: Cell<bool>,
    stop_reason:    Cell<String>,
    timer_manager:  TimerManager<P>,
    pollables:      RefCell<Vec<Box<dyn Pollable<P>>>>, // FIXME
}

impl<P> Default for LiveHub<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> LiveHub<P> {
    pub fn new() -> Self {
        Self {
            timer_manager:  TimerManager::new(2048),
            stop_reason:    Cell::new(String::new()),
            stop_requested: Cell::new(false),
            pollables:      RefCell::new(Vec::new()), // FIXME
        }
    }
}

impl<P: Pipeline> Hub for LiveHub<P> {
    type Pipeline = P;

    type LiveResourceManager = Self;
    type ReplayResourceManager = !;

    fn poll(&self, pipeline: &mut Self::Pipeline) -> ControlFlow<String> {
        self.timer_manager.poll(pipeline);
        for pollable in self.pollables.borrow().iter() {
            pollable.poll(pipeline).expect("Encountered unexpected pollable error") // FIXME
        }

        if util::hint::unlikely(self.stop_requested.get()) {
            ControlFlow::Break(self.stop_reason.take())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn defer<Msg>(&self, _msg: Msg) {
        todo!()
    }

    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn start_timer<Event: Copy>(&self, event: Event, duration: std::time::Duration) {
        self.timer_manager.start_timer(event, self.now() + duration);
    }

    fn stop(&self, reason: String) {
        self.stop_requested.set(true);
        self.stop_reason.set(reason);
    }

    fn as_resource_manager(&self) -> ResourceManager<'_, Self> {
        todo!()
    }
}

impl<P: Pipeline> LiveResourceManager for LiveHub<P> {
    fn register_pollable(&self, pollable: Box<dyn Pollable<Self::Pipeline>>) {
        self.pollables.borrow_mut().push(pollable); // FIXME
    }
}
