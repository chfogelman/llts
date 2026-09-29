use super::*;

#[test]
fn small() {
    let f = TypeErasedFn::<0, _, _>::new(|dst| format!("{}", dst));
    assert_eq!(&f.call(&mut 0), "0");
}

#[test]
#[should_panic]
fn large() {
    let nums = [0i32; 5];
    let f = TypeErasedFn::<2, _>::new(move |s| println!("{s}: {:?}", nums));
    f.call(&mut "nums");
}
