#![feature(min_specialization)]

use framework::{Handler, PollEvent, ThreadBuilder, ThreadStart};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

struct Ping {
    count: i32,
}

struct Pong {
    count: i32,
}

struct Pinger {
    count: i32,
    pair: Arc<AtomicBool>,
}

impl Handler<Pong> for Pinger {
    fn handle(&mut self, _ctx: &mut impl framework::Context, msg: &Pong) {
        println!("Received Pong with count {} on {:?}", msg.count, std::thread::current().id());
        if msg.count > 10 {
            self.pair.store(true, Ordering::Release);
        }
    }
}

impl Handler<PollEvent> for Pinger {
    fn handle(&mut self, ctx: &mut impl framework::Context, _: &PollEvent) {
        ctx.publish(Ping { count: self.count });
        self.count += 1;
    }
}

struct Ponger;

impl Handler<Ping> for Ponger {
    fn handle(&mut self, ctx: &mut impl framework::Context, msg: &Ping) {
        println!("Received Ping with count {} on {:?}", msg.count, std::thread::current().id());
        ctx.publish(Pong { count: msg.count });
    }
}

#[test]
fn ping_pong() {
    let pair = Arc::new(AtomicBool::new(false));
    let threads = ThreadBuilder::build_threads((
        Pinger {
            count: 0,
            pair: pair.clone(),
        },
        Ponger,
    ));
    let _handles = ThreadStart::start(threads);
    std::thread::sleep(std::time::Duration::from_millis(1));
    if !pair.load(Ordering::Acquire) {
        panic!("Failed to finish ");
    }
}
