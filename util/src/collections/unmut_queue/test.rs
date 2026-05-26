use std::cell::Cell;
use std::ops::Drop;
use std::rc::Rc;

use super::UnmutQueue;

#[test]
fn push_n_pop_1() {
    let q = UnmutQueue::with_capacity(1);
    q.push(Box::new([1, 2, 3]) as Box<[i32]>);
    let val = q.pop();
    assert_eq!(*val.unwrap(), vec![1, 2, 3]);
    assert_eq!(q.len(), 0);
}

#[test]
fn resize_from_new() {
    let q = UnmutQueue::new();
    q.push(1);
    assert_eq!(q.pop(), Some(1));
    assert!(q.capacity() > 0);
    assert_eq!(q.len(), 0);
}

#[test]
fn resize() {
    let q = UnmutQueue::with_capacity(3);
    for i in 0..10 {
        q.push(i);
    }
    for i in 0..10 {
        assert_eq!(q.pop(), Some(i));
    }
}

#[test]
fn pop_empty() {
    let q = UnmutQueue::<i32>::with_capacity(5);
    assert_eq!(q.pop(), None);
}

#[test]
fn try_push() {
    let q = UnmutQueue::with_capacity(2);
    assert!(q.try_push(1).is_ok());
    assert!(q.try_push(2).is_ok());
    assert_eq!(q.capacity(), 2);
    assert_eq!(q.try_push(3), Err(3));
}

#[test]
fn drop_1() {
    struct DropChecker(Rc<Cell<bool>>);
    impl Drop for DropChecker {
        fn drop(&mut self) {
            self.0.set(true)
        }
    }
    let check = Rc::new(Cell::new(false));
    let dropped = DropChecker(Rc::clone(&check));
    {
        let q = UnmutQueue::with_capacity(5);
        q.push(dropped);
    }
    assert_eq!(check.get(), true);
}

#[test]
fn drop_empty() {
    // This test doesn't have any assertions but is intended to be checked by miri
    let q = UnmutQueue::<i32>::new();
    drop(q);
    let q = UnmutQueue::<i32>::with_capacity(1);
    drop(q);
}

#[test]
fn push_pop_interleaved() {
    let mut vals = vec![];
    let q1 = &UnmutQueue::with_capacity(4);
    let q2 = q1;
    q1.push(1);
    q1.push(2);
    vals.push(q2.pop().unwrap());
    q1.push(3);
    q2.push(4);
    q1.push(5);
    vals.push(q1.pop().unwrap());
    vals.push(q2.pop().unwrap());
    vals.push(q1.pop().unwrap());
    q2.push(6);
    vals.push(q1.pop().unwrap());
    assert_eq!(vals, [1, 2, 3, 4, 5]);
}

#[test]
fn into_iterator() {
    let q = UnmutQueue::with_capacity(5);
    q.push(Box::new(1));
    q.push(Box::new(2));
    q.push(Box::new(3));
    q.push(Box::new(4));
    q.push(Box::new(5));

    let mut i = 1;
    for x in q {
        assert_eq!(i, *x);
        i += 1;
    }
}

#[test]
fn from_iterator() {
    let q = UnmutQueue::from_iter((1..=5).map(|x| Box::new(x)));
    let mut i = 1;
    while let Some(x) = q.pop() {
        assert_eq!(i, *x);
        i += 1;
    }
}

#[test]
fn drain() {
    let q = UnmutQueue::from_iter((1..=5).map(|x| Box::new(x)));
    let mut i = 1;
    for x in q.drain() {
        assert_eq!(i, *x);
        i += 1;
    }
    assert_eq!(q.len(), 0);
    assert!(q.capacity() >= 5);
    q.push(Box::new(1)); //Shouldn't panic
}

#[test]
fn clone_empty() {
    let q1 = UnmutQueue::<i32>::new();
    let q2 = q1.clone();
    assert!(q1.is_empty());
    assert_eq!(q2.capacity(), 0);
}

#[test]
fn clone() {
    let q1 = UnmutQueue::from_iter(1..=5);
    let q2 = q1.clone();
    for x in 1..=5 {
        assert_eq!(q2.pop(), Some(x))
    }
}

#[test]
fn clone_after_pops() {
    let q1 = UnmutQueue::from_iter(0..10);
    for _ in 0..5 {
        q1.pop();
    }
    let q2 = q1.clone();
    drop(q1);
    for i in 5..10 {
        assert_eq!(q2.pop(), Some(i));
    }
}
