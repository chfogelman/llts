use std::collections::HashMap;

use super::*;

type FS = FixedString<40>;

#[test]
fn test() {
    let hello = FixedString::<40>::try_from("Hello world").unwrap();
    assert_eq!(hello, "Hello world");
    assert_eq!(FixedString::<11, 0>(*b"Hello\0world"), FixedString(*b"Hello"));
}

#[test]
fn test_from() {
    let hello = FS::from(b"Hello world");
    assert_eq!(hello, "Hello world");
}

#[test]
fn test_truncated() {
    assert_eq!(FixedString::<5>::from_truncated("Hel"), "Hel");
    assert_eq!(FixedString::<5>::from_truncated("Hello"), "Hello");
    assert_eq!(FixedString::<5>::from_truncated("Hel\0o"), "Hel");
    assert_eq!(FixedString::<5>::from_truncated("Hello world"), "Hello");
}

#[test]
fn test_fmt() {
    assert_eq!(format!("{}", FS::try_from("Hello world").unwrap()), "Hello world");
    assert_eq!(
        format!("{:?}", FixedString::<12>::try_from("Hello world").unwrap()),
        "\"Hello world\0\"fs<12>"
    );
}

#[test]
fn test_cmp() {
    assert_eq!(FS::from_truncated("Hello world"), FS::from_truncated("Hello world\0xyz"));
    assert_eq!(FS::try_from("Hello world").unwrap(), "Hello world\0xyz");
    assert_ne!(FS::try_from("Hello world").unwrap(), FS::from_truncated("Hello world!"));
    assert_ne!(FS::try_from("Hello world").unwrap(), "Hello world!");
    assert!(FS::try_from("Hello worlc").unwrap() < "Hello world\0xyz");
    assert!(FS::try_from("Hello worl\0z").unwrap() < "Hello world\0xyz");
}

#[test]
#[should_panic]
fn test_len_err() {
    let _hello = FixedString::<10>::try_from("Hello world").unwrap();
}

#[test]
fn test_borrow() {
    let map: HashMap<FS, _> = [("Hello", 1), ("world", 2)]
        .into_iter()
        .map(|(k, v)| (k.try_into().unwrap(), v))
        .collect();
    map.get("Hello".as_bytes()).unwrap();
    map.get("world".as_bytes()).unwrap();
    assert!(map.get("!".as_bytes()).is_none());
}
