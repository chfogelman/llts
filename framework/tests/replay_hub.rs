#![feature(min_specialization)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use framework::PollError;
use framework::driver::Driver;
use framework::messages::*;
use framework::prelude::*;

struct ReplayEvent(i32);

struct TestReplaySource(RefCell<VecDeque<(SystemTime, ReplayEvent)>>);

impl TestReplaySource {
    fn new<I: IntoIterator<Item = (SystemTime, ReplayEvent)>>(events: I) -> Self {
        Self(RefCell::new(VecDeque::from_iter(events.into_iter())))
    }
}

impl<P: Pipeline> ReplaySource<P> for TestReplaySource {
    fn poll(&self, target: &mut P) -> Result<SystemTime, PollError> {
        while let Some(front) = self.0.borrow_mut().pop_front_if(|f| f.0 <= target.get_hub().now()) {
            target.dispatch(&front.1);
        }
        if let Some(front) = self.0.borrow().front() {
            Ok(front.0)
        } else {
            Err(PollError)
        }
    }

    fn peek(&self) -> Result<SystemTime, PollError> {
        if let Some(front) = self.0.borrow_mut().front() {
            Ok(front.0)
        } else {
            Err(PollError)
        }
    }

    fn advance_to(&self, t: SystemTime) -> Result<SystemTime, PollError> {
        while let Some(front) = self.0.borrow_mut().front() {
            if front.0 > t {
                return Ok(front.0);
            }
        }
        Err(PollError)
    }
}

struct Listener {
    next_event: i32,
}

impl Handler<StartEvent> for Listener {
    fn handle(&mut self, ctx: &mut impl Context, _msg: &StartEvent) {
        let ResourceManager::Replay(rm) = ctx.get_hub().as_resource_manager() else {
            panic!("Not a replay resource manager")
        };
        let src = TestReplaySource::new([
            (SystemTime::UNIX_EPOCH + Duration::from_secs(100), ReplayEvent(0)),
            (SystemTime::UNIX_EPOCH + Duration::from_secs(200), ReplayEvent(1)),
            (SystemTime::UNIX_EPOCH + Duration::from_secs(300), ReplayEvent(2)),
        ]);
        rm.register_source(Box::new(src))
    }
}

impl Handler<ReplayEvent> for Listener {
    fn handle(&mut self, _ctx: &mut impl Context, msg: &ReplayEvent) {
        assert_eq!(self.next_event, msg.0, "Unexpected event seqno {}", msg.0);
        self.next_event += 1;
    }
}

impl Handler<Shutdown> for Listener {
    fn handle(&mut self, _ctx: &mut impl Context, _msg: &Shutdown) {
        assert_eq!(self.next_event, 3, "Expected final event seqno: 3");
    }
}

#[test]
fn test() {
    let container = chain_components!(Listener { next_event: 0 });
    let driver = framework::hub::replay::make_driver(container);
    driver.run();
}
