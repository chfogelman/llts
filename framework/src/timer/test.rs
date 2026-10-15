use std::cell::Cell;
use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::resource_manager::ResourceManager;
use crate::{Context, Hub, Pipeline};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TimerEvent(u128);

struct TimerTestPipeline {
    expected: VecDeque<TimerEvent>,
    hub:      Rc<TimerTestHub>,
}

impl Drop for TimerTestPipeline {
    fn drop(&mut self) {
        assert!(self.expected.is_empty());
    }
}

impl Context for TimerTestPipeline {
    type Hub = TimerTestHub;

    fn publish<Msg>(&mut self, _msg: impl crate::envelope::ThreadLocalEnvelope<Msg>) {
        unimplemented!();
    }

    fn get_hub(&self) -> &Self::Hub {
        &self.hub
    }
}

trait Check<Msg> {
    fn check(&mut self, msg: &Msg);
}

impl<Msg> Check<Msg> for TimerTestPipeline {
    default fn check(&mut self, _msg: &Msg) {
        panic!("Received unexpected message: {}", std::any::type_name::<Msg>())
    }
}

impl Check<TimerEvent> for TimerTestPipeline {
    fn check(&mut self, msg: &TimerEvent) {
        let next = self.expected.pop_front();
        assert_eq!(next, Some(*msg));
    }
}

impl Pipeline for TimerTestPipeline {
    fn dispatch<Msg>(&mut self, msg: &Msg) {
        <TimerTestPipeline as Check<Msg>>::check(self, msg);
    }
}

struct TimerTestHub {
    now:    Cell<SystemTime>,
    timers: TimerManager<TimerTestPipeline>,
}

struct TimerTestDriver {
    hub:      Rc<TimerTestHub>,
    pipeline: TimerTestPipeline,
}

impl TimerTestDriver {
    fn new(now: SystemTime) -> Self {
        let hub = Rc::new(TimerTestHub {
            now:    Cell::new(now),
            timers: TimerManager::new(1024),
        });
        Self {
            hub:      Rc::clone(&hub),
            pipeline: TimerTestPipeline {
                expected: VecDeque::new(),
                hub,
            },
        }
    }

    fn advance(&mut self, duration: Duration) {
        self.hub.now.set(self.hub.now.get() + duration)
    }

    fn poll(&mut self) -> ControlFlow<String> {
        self.hub.poll(&mut self.pipeline)
    }
}

impl Hub for TimerTestHub {
    type Pipeline = TimerTestPipeline;

    type LiveResourceManager = !;
    type ReplayResourceManager = !;

    fn poll(&self, pipeline: &mut Self::Pipeline) -> ControlFlow<String> {
        self.timers.poll(pipeline);
        ControlFlow::Continue(())
    }

    fn defer<Msg>(&self, _msg: Msg) {
        unimplemented!()
    }

    fn now(&self) -> SystemTime {
        self.now.get()
    }

    fn start_timer<Event: Copy>(&self, event: Event, duration: std::time::Duration) {
        self.timers.start_timer(event, self.now() + duration)
    }

    fn stop(&self, _reason: String) {
        unimplemented!()
    }

    fn as_resource_manager(&self) -> ResourceManager<'_, Self> {
        unimplemented!()
    }
}

#[test]
fn no_timer() {
    let mut driver = TimerTestDriver::new(SystemTime::UNIX_EPOCH);
    let _ = driver.poll();
    driver.advance(Duration::from_secs(5));
    let _ = driver.poll();
    driver.advance(Duration::from_secs(5));
    assert!(driver.pipeline.expected.is_empty());
}

#[test]
fn one_timer() {
    let mut driver = TimerTestDriver::new(SystemTime::UNIX_EPOCH);
    driver.hub.start_timer(TimerEvent(0), Duration::from_secs(3));
    driver.pipeline.expected.push_back(TimerEvent(0));

    driver.advance(Duration::from_secs(5));
    let _ = driver.poll();

    assert!(driver.pipeline.expected.is_empty());
}

#[test]
fn two_timers() {
    let mut driver = TimerTestDriver::new(SystemTime::UNIX_EPOCH);
    driver.hub.start_timer(TimerEvent(0), Duration::from_secs(3));
    driver.pipeline.expected.push_back(TimerEvent(0));
    driver.hub.start_timer(TimerEvent(1), Duration::from_secs(6));
    driver.pipeline.expected.push_back(TimerEvent(1));

    driver.advance(Duration::from_secs(5));
    let _ = driver.poll();
    assert!(!driver.pipeline.expected.is_empty());

    driver.advance(Duration::from_secs(2));
    let _ = driver.poll();
    assert!(driver.pipeline.expected.is_empty());
}

#[test]
fn two_timers_one_poll() {
    let mut driver = TimerTestDriver::new(SystemTime::UNIX_EPOCH);
    driver.hub.start_timer(TimerEvent(0), Duration::from_secs(2));
    driver.pipeline.expected.push_back(TimerEvent(0));
    driver.hub.start_timer(TimerEvent(1), Duration::from_secs(4));
    driver.pipeline.expected.push_back(TimerEvent(1));

    driver.advance(Duration::from_secs(5));
    let _ = driver.poll();
    assert!(driver.pipeline.expected.is_empty());
}

#[test]
fn timer_exact() {
    let mut driver = TimerTestDriver::new(SystemTime::UNIX_EPOCH);
    driver.hub.start_timer(TimerEvent(0), Duration::from_secs(2));
    driver.pipeline.expected.push_back(TimerEvent(0));

    driver.advance(Duration::from_secs(2));
    let _ = driver.poll();
    assert!(driver.pipeline.expected.is_empty());
}
