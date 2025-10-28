//! Test for derive macro with arena allocation
//!
//! This test verifies that the #[derive(View)] macro generates correct arena-aware code.

use defiant_derive::View;
use defiant::{Arena, Encode};

/// A simple Person message using the derive macro
#[derive(View)]
struct PersonDerived<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(int32, tag = 2)]
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
    let person = PersonDerivedBuilder::decode(&data[..], &arena)
        .unwrap()
        .freeze();

    assert_eq!(person.name, "Alice");
    assert_eq!(person.age, 30);
}

#[test]
fn test_derive_person_encode() {
    let person = PersonDerived {
        name: "Bob",
        age: 25,
    };

    let buf = person.encode_to_vec();

    // Decode it back
    let arena = Arena::new();
    let decoded = PersonDerivedBuilder::decode(&buf[..], &arena)
        .unwrap()
        .freeze();
    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 25);
}

#[test]
fn test_derive_person_new_in() {
    let arena = Arena::new();
    let builder = PersonDerivedBuilder::new_in(&arena);
    let person = builder.freeze();
    assert_eq!(person.name, "");
    assert_eq!(person.age, 0);
}
