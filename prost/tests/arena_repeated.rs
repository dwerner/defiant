//! Test for arena-allocated repeated fields
//!
//! This test demonstrates repeated fields with arena allocation.
//! The corresponding proto would be:
//!
//! ```proto
//! message PersonList {
//!   repeated string names = 1;
//!   repeated int32 ages = 2;
//!   repeated bytes data = 3;
//! }
//! ```

use prost::{Arena, DecodeError, Message};
use prost::encoding::{DecodeContext, WireType, string, int32};
use prost::encoding::bytes as bytes_encoding;
use bytes::{Buf, BufMut};

/// Intermediate struct used during decoding (uses Vec for accumulation)
#[derive(Debug, Default)]
struct PersonListBuilder {
    names: Vec<String>,
    ages: Vec<i32>,
    data: Vec<Vec<u8>>,
}

impl<'arena> Message<'arena> for PersonListBuilder {
    fn new_in(_arena: &'arena Arena) -> Self {
        PersonListBuilder {
            names: Vec::new(),
            ages: Vec::new(),
            data: Vec::new(),
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        // Encode repeated names (tag 1)
        for name in &self.names {
            string::encode(1, name, buf);
        }
        // Encode repeated ages (tag 2)
        for age in &self.ages {
            int32::encode(2, age, buf);
        }
        // Encode repeated data (tag 3)
        for d in &self.data {
            bytes_encoding::encode(3, d, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                // Decode repeated string field
                string::merge_repeated(wire_type, &mut self.names, buf, ctx)
            }
            2 => {
                // Decode repeated int32 field
                int32::merge_repeated(wire_type, &mut self.ages, buf, ctx)
            }
            3 => {
                // Decode repeated bytes field
                bytes_encoding::merge_repeated(wire_type, &mut self.data, buf, ctx)
            }
            _ => {
                // Skip unknown fields
                prost::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        len += string::encoded_len_repeated(1, &self.names);
        len += int32::encoded_len_repeated(2, &self.ages);
        len += bytes_encoding::encoded_len_repeated(3, &self.data);
        len
    }

    fn clear(&mut self) {
        self.names.clear();
        self.ages.clear();
        self.data.clear();
    }
}

/// Final struct with arena-allocated slices
#[derive(Debug)]
struct PersonList<'arena> {
    names: &'arena [&'arena str],
    ages: &'arena [i32],
    data: &'arena [&'arena [u8]],
}

impl PersonListBuilder {
    /// Converts the builder into a final PersonList with arena-allocated slices
    fn into_arena<'arena>(self, arena: &'arena Arena) -> PersonList<'arena> {
        PersonList {
            names: arena.alloc_string_vec(self.names),
            ages: arena.alloc_vec(self.ages),
            data: {
                let byte_slices: Vec<&[u8]> = self.data.iter()
                    .map(|v| arena.alloc_bytes(v))
                    .collect();
                arena.alloc_slice_copy(&byte_slices)
            },
        }
    }
}

#[test]
fn test_repeated_fields_basic() {
    let arena = Arena::new();

    // Create a PersonListBuilder with data
    let builder = PersonListBuilder {
        names: vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()],
        ages: vec![30, 25, 35],
        data: vec![vec![1, 2, 3], vec![4, 5], vec![6]],
    };

    // Encode
    let encoded = builder.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    // Decode into builder
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    // Convert to arena-allocated struct
    let person_list = decoded_builder.into_arena(&arena);

    // Verify
    assert_eq!(person_list.names.len(), 3);
    assert_eq!(person_list.names[0], "Alice");
    assert_eq!(person_list.names[1], "Bob");
    assert_eq!(person_list.names[2], "Charlie");

    assert_eq!(person_list.ages.len(), 3);
    assert_eq!(person_list.ages, &[30, 25, 35]);

    assert_eq!(person_list.data.len(), 3);
    assert_eq!(person_list.data[0], &[1, 2, 3]);
    assert_eq!(person_list.data[1], &[4, 5]);
    assert_eq!(person_list.data[2], &[6]);

    println!("Successfully decoded repeated fields");
    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_repeated_fields_empty() {
    let arena = Arena::new();

    // Create empty PersonListBuilder
    let builder = PersonListBuilder::default();

    // Encode
    let encoded = builder.encode_to_vec();

    // Decode
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.into_arena(&arena);

    // Verify all empty
    assert_eq!(person_list.names.len(), 0);
    assert_eq!(person_list.ages.len(), 0);
    assert_eq!(person_list.data.len(), 0);
}

#[test]
fn test_repeated_fields_single_element() {
    let arena = Arena::new();

    let builder = PersonListBuilder {
        names: vec!["Solo".to_string()],
        ages: vec![42],
        data: vec![vec![99]],
    };

    let encoded = builder.encode_to_vec();
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.into_arena(&arena);

    assert_eq!(person_list.names.len(), 1);
    assert_eq!(person_list.names[0], "Solo");
    assert_eq!(person_list.ages[0], 42);
    assert_eq!(person_list.data[0], &[99]);
}

#[test]
fn test_repeated_fields_large() {
    let arena = Arena::new();

    // Create a large list
    let mut names = Vec::new();
    let mut ages = Vec::new();
    for i in 0..1000 {
        names.push(format!("Person{}", i));
        ages.push(i);
    }

    let builder = PersonListBuilder {
        names,
        ages,
        data: vec![],
    };

    let encoded = builder.encode_to_vec();
    println!("Encoded {} bytes for 1000 items", encoded.len());

    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.into_arena(&arena);

    assert_eq!(person_list.names.len(), 1000);
    assert_eq!(person_list.ages.len(), 1000);

    // Verify some samples
    assert_eq!(person_list.names[0], "Person0");
    assert_eq!(person_list.names[500], "Person500");
    assert_eq!(person_list.names[999], "Person999");

    assert_eq!(person_list.ages[0], 0);
    assert_eq!(person_list.ages[500], 500);
    assert_eq!(person_list.ages[999], 999);

    println!("Arena allocated {} bytes for 1000 items", arena.allocated_bytes());
}

#[test]
fn test_repeated_unicode() {
    let arena = Arena::new();

    let builder = PersonListBuilder {
        names: vec![
            "田中".to_string(),
            "José".to_string(),
            "Müller".to_string(),
            "Владимир".to_string(),
        ],
        ages: vec![30, 25, 35, 40],
        data: vec![],
    };

    let encoded = builder.encode_to_vec();
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.into_arena(&arena);

    assert_eq!(person_list.names[0], "田中");
    assert_eq!(person_list.names[1], "José");
    assert_eq!(person_list.names[2], "Müller");
    assert_eq!(person_list.names[3], "Владимир");
}
