use std::sync::atomic::{AtomicU32, AtomicU64};

use crate::align::CacheLineAligned;

#[repr(align(64))]
struct Block {
    generation: AtomicU32,
    storage:    [u8; 60],
}

const _: () = assert!(std::mem::size_of::<Block>() == 64);

impl Default for Block {
    fn default() -> Self {
        Block {
            generation: AtomicU32::new(0),
            storage:    [0; _],
        }
    }
}

struct Queue {
    items:         Box<[Block]>,
    block_counter: CacheLineAligned<AtomicU64>,
}
