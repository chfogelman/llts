use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;

use util::queue::broadcast;

#[derive(Clone, Copy)]
struct SimpleLcg(u64);

impl SimpleLcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        return self.0;
    }
}

const NUM_ITERS: i64 = if cfg!(miri) {
    100
} else if !cfg!(debug_assertions) {
    10_000_000
} else {
    10_000
};

const SIZE: usize = if cfg!(miri) { 1 << 5 } else { 1 << 20 };

#[test]
fn test() {
    let (tx, rx) = broadcast::channel(SIZE).expect("valid capacity");
    let done_count = &AtomicU8::new(0);
    thread::scope(|s| {
        let mut rng = SimpleLcg(0);
        let threads: Vec<_> = (0..4)
            .map(|tid| {
                let rx = rx.clone();
                s.spawn(move || {
                    let mut count = 0_i64;
                    let mut lagged = false;
                    while count < NUM_ITERS {
                        match rx.recv() {
                            Ok(val) => {
                                if lagged {
                                    rng = SimpleLcg(val);
                                    lagged = false;
                                } else if val != rng.next() {
                                    panic!("Mismatch at {}", val);
                                }
                                count += 1;
                            },
                            Err(_) => {
                                println!("Thread {tid} lagged at {count}");
                                lagged = true;
                            },
                        }
                    }
                    done_count.fetch_add(1, Ordering::Relaxed);
                    println!("Done!");
                })
            })
            .collect();

        s.spawn(move || {
            while done_count.load(Ordering::Relaxed) < 4 {
                tx.send(rng.next());
            }
        })
        .join()
        .unwrap();

        for thread in threads {
            thread.join().unwrap();
        }
    });
}
