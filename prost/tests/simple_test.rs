//! Simple test without repeated fields

use prost_derive::Message;

#[derive(Message)]
struct SimplePerson<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(int32, tag = "2")]
    age: i32,
}

#[test]
fn test_simple() {
    use prost::Arena;

    let arena = Arena::new();

    let mut msg = SimplePersonMessage::new_in(&arena);
    msg.set_name("Alice");
    msg.set_age(30);

    assert_eq!(msg.name(), "Alice");
    assert_eq!(msg.age(), 30);
}
