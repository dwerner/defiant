//! Simple test without repeated fields

use defiant_derive::View;

#[derive(View)]
struct SimplePerson<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(int32, tag = "2")]
    age: i32,
}

#[test]
fn test_simple() {
    use defiant::Arena;

    let arena = Arena::new();

    let mut msg = SimplePersonBuilder::new_in(&arena);
    msg.set_name("Alice");
    msg.set_age(30);

    assert_eq!(msg.name(), "Alice");
    assert_eq!(msg.age(), 30);
}
