#![feature(min_specialization)]

use framework::{Handler, PollEvent, ThreadBuilder, ThreadStart, publish};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

#[derive(Clone, Copy)]
struct Ping {
    count: i32,
}

#[derive(Clone, Copy)]
struct Pong {
    count: i32,
}

struct Pinger {
    count: i32,
    flag: Arc<AtomicBool>,
}

impl Handler<Pong> for Pinger {
    fn handle(&mut self, _ctx: &mut impl framework::Context, msg: &Pong) {
        println!("Received Pong with count {} on {:?}", msg.count, std::thread::current().id());
        if msg.count > 10 {
            self.flag.store(true, Ordering::Release);
        }
    }
}

impl Handler<PollEvent> for Pinger {
    fn handle(&mut self, ctx: &mut impl framework::Context, _: &PollEvent) {
        publish!(ctx, Ping { count: self.count });
        self.count += 1;
    }
}

struct Ponger;

impl Handler<Ping> for Ponger {
    fn handle(&mut self, ctx: &mut impl framework::Context, msg: &Ping) {
        println!("Received Ping with count {} on {:?}", msg.count, std::thread::current().id());
        publish!(ctx, Pong { count: msg.count });
    }
}

#[test]
fn ping_pong() {
    let duration = std::time::Duration::from_micros(250);
    let flag = Arc::new(AtomicBool::new(false));
    let threads = ThreadBuilder::build_threads((
        Pinger {
            count: 0,
            flag: flag.clone(),
        },
        Ponger,
    ));
    let _handles = ThreadStart::start(threads);
    std::thread::sleep(duration);
    if !flag.load(Ordering::Acquire) {
        panic!("Failed to finish within {:?}", duration);
    }
}
