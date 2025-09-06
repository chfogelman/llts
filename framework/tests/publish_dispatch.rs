#![feature(min_specialization)]

use framework::{Handler, PollEvent, ThreadBuilder, ThreadStart, publish};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

struct ThreadTheFirst {
    sent: bool,
}

impl Handler<PollEvent> for ThreadTheFirst {
    fn handle(&mut self, ctx: &mut impl framework::Context, _: &PollEvent) {
        if !self.sent {
            publish!(ctx, 0i32);
            publish!(ctx, Arc::new(0i32));
        }
    }
}

struct ThreadTheSecond{
    received_i32: bool,
    finished: Arc<AtomicBool>,
}

impl Handler<i32> for ThreadTheSecond {
    fn handle(&mut self, _: &mut impl framework::Context, _: &i32) {
        assert!(!self.received_i32 && !self.finished.load(Ordering::Relaxed));
        self.received_i32 = true;
    }
}

impl Handler<Arc<i32>> for ThreadTheSecond {
    fn handle(&mut self, _: &mut impl framework::Context, _: &Arc<i32>) {
        assert!(self.received_i32 && !self.finished.load(Ordering::Acquire));
        self.finished.store(true, Ordering::Release);
    }
}

#[test]
fn test() {
    let duration = std::time::Duration::from_micros(250);
    let flag = Arc::new(AtomicBool::new(false));
    let threads = ThreadBuilder::build_threads((
            ThreadTheFirst {
                sent: false,
            },
            ThreadTheSecond {
                received_i32: false,
                finished: flag.clone(),
            },
    ));
    let _handles = ThreadStart::start(threads);
    std::thread::sleep(duration);
    if !flag.load(Ordering::Acquire) {
        panic!("Failed to finish within {:?}", duration);
    }
}
