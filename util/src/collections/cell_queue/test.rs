use std::cell::Cell;
use std::ops::Drop;
use std::rc::Rc;

use super::UnmutQueue;

#[test]
fn push_n_pop_1() {
    let q = UnmutQueue::new(5);
    q.push(vec![1, 2, 3]);
    let val = q.pop();
    assert_eq!(val, Some(vec![1, 2, 3]));
    assert_eq!(q.len(), 0);
}

#[test]
fn pop_empty() {
    let q = UnmutQueue::<i32>::new(5);
    assert_eq!(q.pop(), None);
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
        let q = UnmutQueue::new(5);
        q.push(dropped);
    }
    assert_eq!(check.get(), true);
}

#[test]
fn push_pop_interleaved() {
    let mut vals = vec![];
    let q1 = &UnmutQueue::new(4);
    let q2 = q1;
    q1.push(1);
    q1.push(2);
    vals.push(q2.pop().unwrap());
    q1.push(3);
    q2.push(4);
    vals.push(q1.pop().unwrap());
    vals.push(q2.pop().unwrap());
    vals.push(q1.pop().unwrap());
    q1.push(5);
    q2.push(6);
    vals.push(q1.pop().unwrap());
    assert_eq!(vals, [1, 2, 3, 4, 5]);
}

#[test]
fn into_iterator() {
    let q = UnmutQueue::new(5);
    q.push(1);
    q.push(2);
    q.push(3);
    q.push(4);
    q.push(5);

    let mut i = 1;
    for x in q {
        assert_eq!(i, x);
        i += 1;
    }
}

#[test]
fn from_iterator() {
    let q = UnmutQueue::from_iter([1, 2, 3, 4, 5, 6, 7]);
    let mut i = 1;
    while let Some(x) = q.pop() {
        assert_eq!(i, x);
        i += 1;
    }
}

#[test]
fn drain() {
    let q = UnmutQueue::from_iter([1, 2, 3, 4, 5]);
    let mut i = 1;
    for x in q.drain() {
        assert_eq!(i, x);
        i += 1;
    }
    assert_eq!(q.len(), 0);
    assert_eq!(q.capacity(), 5);
    q.push(1); //Shouldn't panic
}
