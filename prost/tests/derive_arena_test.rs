//! Test for derive macro with arena allocation
//!
//! This test verifies that the #[derive(Message)] macro generates correct arena-aware code.

use prost::{Arena, Message};

/// A simple Person message using the derive macro
#[derive(prost_derive::Message)]
struct PersonDerived<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,
    #[prost(int32, tag = "2")]
    age: i32,
}

#[test]
fn test_derive_person_decode() {
    // Create test data: name="Alice", age=30
    let mut data = Vec::new();
    // Tag 1 (string): field=1, wire_type=2
    data.extend_from_slice(&[0x0a, 0x05]); // tag=1, len=5
    data.extend_from_slice(b"Alice");
    // Tag 2 (int32): field=2, wire_type=0, value=30
    data.extend_from_slice(&[0x10, 0x1e]); // tag=2, value=30

    let arena = Arena::new();
    let person = PersonDerivedMessage::decode(&data[..], &arena).unwrap();

    assert_eq!(person.name, "Alice");
    assert_eq!(person.age, 30);
}

#[test]
fn test_derive_person_encode() {
    let arena = Arena::new();
    let name = arena.alloc_str("Bob");
    let person = PersonDerived { name, age: 25 };

    let mut buf = Vec::new();
    person.encode(&mut buf).unwrap();

    // Decode it back
    let arena2 = Arena::new();
    let decoded = PersonDerivedMessage::decode(&buf[..], &arena2).unwrap();
    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 25);
}

#[test]
fn test_derive_person_new_in() {
    let arena = Arena::new();
    let person = PersonDerivedMessage::new_in(&arena);
    assert_eq!(person.name(), "");
    assert_eq!(person.age(), 0);
}
