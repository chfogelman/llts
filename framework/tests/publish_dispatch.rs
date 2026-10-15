#![feature(min_specialization)]
use std::sync::Arc;

use framework::PollableFn;
use framework::messages::StartEvent;
use framework::prelude::*;

struct PollEvent;

struct ThreadTheFirst;

impl Handler<StartEvent> for ThreadTheFirst {
    fn handle(&mut self, ctx: &mut impl Context, _: &StartEvent) {
        if let ResourceManager::Live(mgr) = ctx.get_hub().as_resource_manager() {
            mgr.register_pollable(Box::new(PollableFn(|| Ok(PollEvent))))
        }
    }
}

impl Handler<PollEvent> for ThreadTheFirst {
    fn handle(&mut self, ctx: &mut impl framework::Context, _: &PollEvent) {
        publish!(ctx, 0i32);
        publish!(ctx, Arc::new(0i32));
        ctx.get_hub().stop("Done".into());
    }
}

struct ThreadTheSecond {
    received_i32: bool,
    finished:     bool,
}

impl Handler<i32> for ThreadTheSecond {
    fn handle(&mut self, _: &mut impl framework::Context, _: &i32) {
        assert!(!self.received_i32 && !self.finished);
        self.received_i32 = true;
    }
}

impl Handler<Arc<i32>> for ThreadTheSecond {
    fn handle(&mut self, _: &mut impl framework::Context, _: &Arc<i32>) {
        assert!(self.received_i32 && !self.finished);
    }
}

#[test]
fn test() {
    std::panic::set_hook(Box::new(|payload| {
        if let Some(location) = payload.location() {
            eprintln!("{},{}", location.file(), location.line());
        }
        if let Some(s) = payload.payload_as_str() {
            eprintln!("{}", s);
        }
        std::process::exit(-1)
    }));
    let _handles = ThreadBuilder::start_threads((
        chain_components!(ThreadTheFirst),
        chain_components!(ThreadTheSecond {
            received_i32: false,
            finished:     false,
        }),
    ));
}
