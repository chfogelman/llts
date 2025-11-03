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

#[test]
fn test() {
    let (tx, rx) = broadcast::channel(1 << 16);
    let mut rng = SimpleLcg(0);
    for _ in 0..4 {
        let rx = rx.clone();
        std::thread::spawn(move || {
            let mut count = 0_i64;
            let mut lagged = false;
            while count < NUM_ITERS {
                match rx.recv() {
                    Ok(val) => {
                        if lagged {
                            rng = SimpleLcg(val);
                            lagged = false;
                        } else if val != rng.next() {
                            eprintln!("Failed");
                            std::process::exit(-1);
                        }
                        count += 1;
                    },
                    Err(_) => {
                        println!("Lagged! {}", count);
                        lagged = true;
                    },
                }
            }
            std::process::exit(0);
        });
    }
    thread::spawn(move || {
        loop {
            tx.send(rng.next());
        }
    })
    .join()
    .unwrap();
}
