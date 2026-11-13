#![feature(min_specialization)]
struct PollEvent;

use framework::PollableFn;
use framework::messages::StartEvent;
use framework::prelude::*;

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
}

impl Handler<StartEvent> for Pinger {
    fn handle(&mut self, ctx: &mut impl Context, _: &StartEvent) {
        if let ResourceManager::Live(mgr) = ctx.get_hub().as_resource_manager() {
            mgr.register_pollable(Box::new(PollableFn(|| Ok(PollEvent))));
        }
    }
}

impl Handler<Pong> for Pinger {
    fn handle(&mut self, ctx: &mut impl Context, msg: &Pong) {
        println!("Received Pong with count {} on {:?}", msg.count, std::thread::current().id());
        if msg.count > 10 {
            ctx.get_hub().stop("Received at least 10 Pongs".into())
        }
    }
}

impl Handler<PollEvent> for Pinger {
    fn handle(&mut self, ctx: &mut impl Context, _: &PollEvent) {
        publish!(ctx, Ping { count: self.count });
        self.count += 1;
    }
}

struct Ponger;

impl Handler<Ping> for Ponger {
    fn handle(&mut self, ctx: &mut impl Context, msg: &Ping) {
        println!("Received Ping with count {} on {:?}", msg.count, std::thread::current().id());
        publish!(ctx, Pong { count: msg.count });
    }
}

#[test]
fn ping_pong() {
    let pinger = chain_components!(Pinger { count: 0 });
    let ponger = chain_components!(Ponger);
    let handles = ThreadBuilder::start_threads((pinger, ponger));
    let results = handles.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(results.contains(&"Received at least 10 Pongs".into()))
}
