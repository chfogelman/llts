use super::*;

#[test]
fn test_send() {
    let (tx, rx) = broadcast::channel(2).expect("valid capacity");
    tx.send(420);
    tx.send(69);

    assert_eq!(rx.recv().unwrap(), 420);
    assert_eq!(rx.recv().unwrap(), 69);

    tx.send(1);

    assert_eq!(rx.recv().unwrap(), 1);
}

#[test]
fn test_lag() {
    let (tx, rx) = broadcast::channel(2).expect("valid capacity");

    tx.send(1);
    tx.send(2);
    tx.send(3);

    assert_eq!(rx.recv(), Err(broadcast::Lagged));

    tx.send(4);
    assert_eq!(rx.recv().unwrap(), 4);
}

#[test]
fn test_try_recv() {
    let (tx, rx) = broadcast::channel(2).expect("valid capacity");

    tx.send(1);

    assert_eq!(rx.try_recv().unwrap().unwrap(), 1);
    assert_eq!(rx.try_recv(), None);

    tx.send(2);
    tx.send(3);
    tx.send(4);

    assert_eq!(rx.try_recv(), Some(Err(broadcast::Lagged)));
    assert_eq!(rx.try_recv(), None);
}

#[test]
fn test_small_capacity() {
    assert!(broadcast::channel::<i32>(1).is_err());
}

#[test]
fn test_bad_capacity() {
    assert!(broadcast::channel::<i32>(5).is_err());
}
