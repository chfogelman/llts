use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super as tb;
use super::TripleBuffer;

fn make_arced_tb<T: Default + Clone>() -> (tb::Writer<Arc<TripleBuffer<T>>>, tb::Reader<Arc<TripleBuffer<T>>>) {
    let arc = Arc::new(TripleBuffer::new(Default::default()));
    tb::new(arc)
}

#[test]
fn write_simple() {
    let (mut wr, mut rd) = make_arced_tb::<i32>();
    for i in 0..10 {
        *wr.get_mut() = i;
        assert_eq!(*rd.read(), i);
    }
}

#[test]
fn write_coalesced() {
    let (mut wr, mut rd) = make_arced_tb::<i32>();
    for i in 0..=10 {
        *wr.get_mut() = i;
    }
    assert_eq!(*rd.read(), 10);
}

#[test]
fn read_newer() {
    let (mut wr, mut rd) = make_arced_tb();
    assert!(rd.read_newer().is_none());
    *wr.get_mut() = 1000;
    assert_eq!(rd.read_newer(), Some(&1000));
}

#[test]
fn read_many() {
    let (mut wr, mut rd) = make_arced_tb();
    *wr.get_mut() = 1000;
    for _ in 0..10 {
        assert_eq!(*rd.read(), 1000);
    }
}

#[test]
fn threaded() {
    static DONE: AtomicBool = AtomicBool::new(false);
    let (mut wr, mut rd) = make_arced_tb();
    let rx = std::thread::spawn(move || {
        loop {
            if let Some(&x) = rd.read_newer() {
                if x > 1000 {
                    DONE.store(true, Ordering::Release);
                    return x;
                }
            }
        }
    });
    let tx = std::thread::spawn(move || {
        for i in 0.. {
            *wr.get_mut() = i;
            if DONE.load(Ordering::Acquire) {
                return;
            }
        }
    });
    let final_value = rx.join().unwrap();
    let _ = tx.join().unwrap();

    assert!(final_value > 1000);
}

#[test]
fn non_arced() {
    let tb = TripleBuffer::new(0);
    let (mut wr, mut rd) = tb::new(&tb);
    *wr.get_mut() = 1000;
    assert_eq!(*rd.read(), 1000);
}

#[test]
#[should_panic]
fn cannot_acquire_twice() {
    let tb = TripleBuffer::new(0);
    let _rw1 = tb::new(&tb);
    let _rw2 = tb::new(&tb);
}
