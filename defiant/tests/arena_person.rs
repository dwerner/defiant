//! Test for arena-allocated Person message

use defiant::{Arena, Encode, Message};

/// A simple Person message with arena-allocated fields.
#[derive(Message)]
struct Person<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(int32, tag = 2)]
    age: i32,
}

#[test]
fn test_person_encode_decode() {
    let arena = Arena::new();

    // Create a person
    let person = Person {
        name: "Alice",
        age: 30,
    };

    // Encode to bytes
    let bytes = person.encode_to_vec();
    println!("Encoded {} bytes", bytes.len());

    // Decode from bytes
    let decoded = PersonBuilder::decode(&bytes[..], &arena)
        .expect("Failed to decode")
        .freeze();

    // Verify
    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);

    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_person_default_values() {
    let arena = Arena::new();

    // Create a person with default values
    let person = Person { name: "", age: 0 };

    let bytes = person.encode_to_vec();
    let decoded = PersonBuilder::decode(&bytes[..], &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.name, "");
    assert_eq!(decoded.age, 0);
}

#[test]
fn test_person_unicode() {
    let arena = Arena::new();

    // Test with Unicode name
    let person = Person {
        name: "José García-Müller (田中)",
        age: 25,
    };

    let bytes = person.encode_to_vec();
    let decoded = PersonBuilder::decode(&bytes[..], &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.name, "José García-Müller (田中)");
    assert_eq!(decoded.age, 25);
}
