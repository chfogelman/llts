#![feature(min_specialization)]
struct PollEvent;

use framework::{Context, Handler, Hub, PollableFn, StartEvent, ThreadBuilder, chain_components, publish};

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
        ctx.get_hub().register_pollable(Box::new(PollableFn(|| Ok(PollEvent))));
    }
}

impl Handler<Pong> for Pinger {
    fn handle(&mut self, ctx: &mut impl Context, msg: &Pong) {
        println!("Received Pong with count {} on {:?}", msg.count, std::thread::current().id());
        if msg.count > 10 {
            ctx.get_hub().stop("Done".into())
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
    std::panic::set_hook(Box::new(|payload| {
        if let Some(location) = payload.location() {
            eprintln!("{},{}", location.file(), location.line());
        }
        if let Some(s) = payload.payload_as_str() {
            eprintln!("{}", s);
        }
        std::process::exit(-1)
    }));
    let pinger = chain_components!(Pinger { count: 0 });
    let ponger = chain_components!(Ponger);
    let _handles = ThreadBuilder::start_threads((pinger, ponger));
}
