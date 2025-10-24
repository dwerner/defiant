//! Test for arena-allocated Person message
//!
//! This test demonstrates a manually written protobuf message with arena allocation.
//! The corresponding proto would be:
//!
//! ```proto
//! message Person {
//!   string name = 1;
//!   int32 age = 2;
//! }
//! ```

use prost::{Arena, DecodeError, Message};
use prost::encoding::{DecodeContext, WireType, string, int32};
use bytes::{Buf, BufMut};

/// A simple Person message with arena-allocated fields.
#[derive(Debug)]
struct Person<'arena> {
    name: &'arena str,
    age: i32,
}

impl<'arena> Default for Person<'arena> {
    fn default() -> Self {
        Person {
            name: "",
            age: 0,
        }
    }
}

impl<'arena> Message<'arena> for Person<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Person {
            name: "",
            age: 0,
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        // Encode name (tag 1, string)
        if !self.name.is_empty() {
            string::encode(1, &self.name.to_string(), buf);
        }
        // Encode age (tag 2, int32)
        if self.age != 0 {
            int32::encode(2, &self.age, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                // Decode string field using arena
                self.name = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            2 => {
                // Decode int32 field
                int32::merge(wire_type, &mut self.age, buf, ctx)
            }
            _ => {
                // Skip unknown fields
                prost::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.name.is_empty() {
            len += string::encoded_len(1, &self.name.to_string());
        }
        if self.age != 0 {
            len += int32::encoded_len(2, &self.age);
        }
        len
    }
}

#[test]
fn test_person_encode_decode() {
    // Create a person
    let arena = Arena::new();

    // Encode a person
    let person = Person {
        name: "Alice",
        age: 30,
    };

    let encoded = person.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    // Decode the person using arena
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person");

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);

    println!("Successfully decoded person: name={}, age={}", decoded.name, decoded.age);
}

#[test]
fn test_person_empty_string() {
    let arena = Arena::new();

    // Encode a person with empty name
    let person = Person {
        name: "",
        age: 42,
    };

    let encoded = person.encode_to_vec();

    // Decode
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person");

    assert_eq!(decoded.name, "");
    assert_eq!(decoded.age, 42);
}

#[test]
fn test_person_zero_age() {
    let arena = Arena::new();

    // Encode a person with zero age
    let person = Person {
        name: "Bob",
        age: 0,
    };

    let encoded = person.encode_to_vec();

    // Decode
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person");

    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 0);
}

#[test]
fn test_person_unicode() {
    let arena = Arena::new();

    // Encode a person with Unicode name
    let person = Person {
        name: "田中太郎",
        age: 25,
    };

    let encoded = person.encode_to_vec();

    // Decode
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person");

    assert_eq!(decoded.name, "田中太郎");
    assert_eq!(decoded.age, 25);
}

#[test]
fn test_person_arena_reuse() {
    let mut arena = Arena::new();

    // Decode first person
    let person1 = Person {
        name: "Alice",
        age: 30,
    };
    let encoded1 = person1.encode_to_vec();
    let decoded1 = Person::decode(encoded1.as_slice(), &arena)
        .expect("Failed to decode person 1");

    assert_eq!(decoded1.name, "Alice");
    assert_eq!(decoded1.age, 30);

    let first_alloc = arena.allocated_bytes();
    println!("Allocated after first decode: {} bytes", first_alloc);
    assert!(first_alloc > 0, "Arena should have allocated memory");

    // Decode second person (arena accumulates)
    let person2 = Person {
        name: "Bob",
        age: 40,
    };
    let encoded2 = person2.encode_to_vec();
    let decoded2 = Person::decode(encoded2.as_slice(), &arena)
        .expect("Failed to decode person 2");

    assert_eq!(decoded2.name, "Bob");
    assert_eq!(decoded2.age, 40);

    let second_alloc = arena.allocated_bytes();
    println!("Allocated after second decode: {} bytes", second_alloc);
    // Note: allocated_bytes() returns capacity, not used bytes,
    // so it might not grow if we're still in the first chunk

    // Reset arena
    arena.reset();
    let after_reset = arena.allocated_bytes();
    println!("Allocated after reset: {} bytes", after_reset);

    // Decode third person (arena reused)
    let person3 = Person {
        name: "Charlie",
        age: 50,
    };
    let encoded3 = person3.encode_to_vec();
    let decoded3 = Person::decode(encoded3.as_slice(), &arena)
        .expect("Failed to decode person 3");

    assert_eq!(decoded3.name, "Charlie");
    assert_eq!(decoded3.age, 50);
}
