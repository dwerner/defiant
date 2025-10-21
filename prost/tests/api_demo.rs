//! Demonstration of the arena-based Message API

use prost::{Arena, MessageView};
use prost_derive::Message;

#[derive(Message)]
struct Person<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(int32, tag = "2")]
    age: i32,

    #[prost(string, repeated, tag = "3")]
    emails: &'arena [&'arena str],
}

#[test]
fn demo_builder_pattern() {
    let arena = Arena::new();

    // Create builder using MessageView trait
    let mut person = <Person as MessageView>::Builder::new_in(&arena);

    // Build the message
    person.set_name("Alice");
    person.set_age(30);
    person.push_emails("alice@example.com");
    person.push_emails("alice@work.com");

    // Inspect state while building using getters
    println!("Building: name={}, emails={:?}", person.name(), person.emails());

    // Continue building
    person.push_emails("alice@home.com");

    // Consume builder to get final view
    let final_person = person.into_view();
    assert_eq!(final_person.name, "Alice");
    assert_eq!(final_person.age, 30);
    assert_eq!(final_person.emails.len(), 3);
}

#[test]
fn demo_encode_decode() {
    let arena1 = Arena::new();

    // Build a message
    let mut builder = PersonMessage::new_in(&arena1);
    builder.set_name("Bob");
    builder.set_age(25);
    builder.push_emails("bob@example.com");

    let person = builder.into_view();

    // Encode
    let mut buf = Vec::new();
    person.encode(&mut buf).unwrap();

    // Decode into new arena
    let arena2 = Arena::new();
    let decoded = PersonMessage::decode(&buf[..], &arena2).unwrap();

    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 25);
    assert_eq!(decoded.emails, &["bob@example.com"]);
}
