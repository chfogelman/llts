#![feature(min_specialization)]

use framework::container::{self, Container};
use framework::{Handler, PollEvent, ThreadBuilder, publish};

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

impl Handler<Pong> for Pinger {
    fn handle(&mut self, ctx: &mut impl framework::Context, msg: &Pong) {
        println!("Received Pong with count {} on {:?}", msg.count, std::thread::current().id());
        if msg.count > 10 {
            ctx.stop()
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
    //std::panic::set_hook(Box::new(|payload| {
    //    if let Some(location) = payload.location() {
    //        eprintln!("{},{}", location.file(), location.line());
    //    }
    //    if let Some(s) = payload.payload().downcast_ref::<String>() {
    //        eprintln!("{}", s);
    //    }
    //    if let Some(s) = payload.payload().downcast_ref::<&str>() {
    //        eprintln!("{}", s);
    //    }
    //    std::process::exit(-1)
    //}));
    let pinger = container::PipelineBuilder.with(Pinger { count: 0 });
    let ponger = container::PipelineBuilder.with(Ponger);
    let _handles = ThreadBuilder::start_threads((pinger, ponger));
}
