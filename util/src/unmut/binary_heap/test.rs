use std::rc::Rc;

use super::*;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct DropChecker(Rc<Cell<bool>>);
impl Drop for DropChecker {
    fn drop(&mut self) {
        self.0.set(true)
    }
}
#[test]
fn new() {
    let heap = BinaryHeap::<i32>::new();
    assert_eq!(heap.pop(), None);
}

#[test]
fn with_capacity() {
    let heap = BinaryHeap::<i32>::with_capacity(100);
    assert_eq!(heap.pop(), None);
}

#[test]
fn one_elem() {
    let heap = BinaryHeap::with_capacity(1);
    heap.push(1);
    assert_eq!(heap.pop(), Some(1));
    assert_eq!(heap.pop(), None);
}

#[test]
fn is_max_heap() {
    let heap = BinaryHeap::with_capacity(2);
    for x in [1, 2] {
        heap.push(x);
    }
    assert_eq!(heap.pop(), Some(2));
    assert_eq!(heap.pop(), Some(1));
}

#[test]
fn multiple_reallocs() {
    let heap = BinaryHeap::new();
    for i in 0..=10 {
        heap.push(i);
    }
    for i in (0..=10).rev() {
        assert_eq!(heap.pop(), Some(i));
    }
}

#[test]
fn interleaved_push_pop() {
    let heap = BinaryHeap::new();
    for i in 0..10 {
        heap.push(i);
    }
    for i in (5..10).rev() {
        assert_eq!(heap.pop(), Some(i));
    }
    for i in 5..10 {
        heap.push(i);
    }
    for i in (0..10).rev() {
        assert_eq!(heap.pop(), Some(i));
    }
}

#[test]
fn handles_duplicates() {
    let heap = BinaryHeap::new();
    for i in (0..10).chain(5..10) {
        heap.push(i)
    }
    for i in (5..10).rev() {
        assert_eq!(heap.pop(), Some(i));
        assert_eq!(heap.pop(), Some(i));
    }
}

#[test]
fn pop_if() {
    let heap = BinaryHeap::new();
    heap.push(10);
    assert_eq!(unsafe { heap.pop_if(|&x| x < 5) }, None);
    assert_eq!(unsafe { heap.pop_if(|&x| x < 15) }, Some(10));
}

#[test]
fn drop_1() {
    let check = Rc::new(Cell::new(false));
    let dropped = DropChecker(Rc::clone(&check));
    {
        let q = BinaryHeap::with_capacity(5);
        q.push(dropped);
    }
    assert_eq!(check.get(), true);
}

#[test]
fn drop_empty() {
    // This test doesn't have any assertions but is intended to be checked by miri
    let q = BinaryHeap::<i32>::new();
    drop(q);
    let q = BinaryHeap::<i32>::with_capacity(1);
    drop(q);
}

#[test]
fn clear() {
    let check = Rc::new(Cell::new(false));
    let dropped = DropChecker(Rc::clone(&check));
    let q = BinaryHeap::with_capacity(5);
    q.push(dropped);
    q.clear();
    assert_eq!(check.get(), true);
}
