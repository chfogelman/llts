use std::cell::{Cell, RefCell};
use std::time::SystemTime;

use crate::timer::TimerManager;
use crate::{Hub, Pipeline, Pollable};

pub struct LiveHub<P> {
    stop_requested: Cell<bool>,
    timer_manager:  TimerManager<P>,
    pollables:      RefCell<Vec<Box<dyn Pollable<P>>>>, // FIXME
}

impl<P> LiveHub<P> {
    pub fn new() -> Self {
        Self {
            timer_manager:  TimerManager::new(2048),
            stop_requested: Cell::new(false),
            pollables:      RefCell::new(Vec::new()), // FIXME
        }
    }

    pub fn stop_requested(&self) -> bool {
        self.stop_requested.get()
    }
}

impl<P: Pipeline> Hub for LiveHub<P> {
    type Pipeline = P;

    fn poll(&self, pipeline: &mut Self::Pipeline) {
        self.timer_manager.poll(pipeline);
        for pollable in self.pollables.borrow().iter() {
            pollable.poll(pipeline).expect("Encountered unexpected pollable error") // FIXME
        }
    }

    fn register_pollable(&self, pollable: Box<dyn Pollable<Self::Pipeline>>) {
        self.pollables.borrow_mut().push(pollable); // FIXME
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
        eprintln!("Stopping for reason: {}", reason);
        self.stop_requested.set(true);
    }
}
