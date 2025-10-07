#![feature(min_specialization)]
use std::sync::Arc;

use framework::{Handler, PollEvent, ThreadBuilder, publish};

struct ThreadTheFirst;

impl Handler<PollEvent> for ThreadTheFirst {
    fn handle(&mut self, ctx: &mut impl framework::Context, _: &PollEvent) {
        publish!(ctx, 0i32);
        publish!(ctx, Arc::new(0i32));
        ctx.stop();
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
    let _handles = ThreadBuilder::start_threads((
        ThreadTheFirst,
        ThreadTheSecond {
            received_i32: false,
            finished:     false,
        },
    ));
}
