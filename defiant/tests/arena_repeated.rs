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

/// Builder struct using regular Vec for accumulation during decoding
/// The arena-borrowed data is transmuted to 'static for storage (safe because arena outlives builder)
#[derive(Debug, Default)]
struct PersonListBuilder {
    names: Vec<&'static str>,
    ages: Vec<i32>,
    data: Vec<&'static [u8]>,
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
        string::encode_repeated(1, &self.names, buf);
        // Encode repeated ages (tag 2)
        int32::encode_repeated(2, &self.ages, buf);
        // Encode repeated data (tag 3)
        bytes_encoding::encode_repeated(3, &self.data, buf);
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
                // Decode repeated string field
                let name = string::merge_arena(wire_type, buf, arena, ctx)?;
                // Transmute from 'arena to 'static (safe because arena outlives builder)
                let name_static: &'static str = unsafe { std::mem::transmute(name) };
                self.names.push(name_static);
                Ok(())
            }
            2 => {
                // Decode repeated int32 field
                int32::merge_repeated(wire_type, &mut self.ages, buf, ctx)
            }
            3 => {
                // Decode repeated bytes field
                let bytes = bytes_encoding::merge_arena(wire_type, buf, arena, ctx)?;
                // Transmute from 'arena to 'static (safe because arena outlives builder)
                let bytes_static: &'static [u8] = unsafe { std::mem::transmute(bytes) };
                self.data.push(bytes_static);
                Ok(())
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

}

/// Final struct with arena-allocated slices
#[derive(Debug)]
struct PersonList<'arena> {
    names: &'arena [&'arena str],
    ages: &'arena [i32],
    data: &'arena [&'arena [u8]],
}

impl PersonListBuilder {
    /// Converts the builder into a final PersonList with slices
    /// Transmutes back from 'static to the arena lifetime (safe because data is arena-allocated)
    fn freeze<'arena>(self) -> PersonList<'arena> {
        let names: &'static [&'static str] = self.names.leak();
        let ages: &'static [i32] = self.ages.leak();
        let data: &'static [&'static [u8]] = self.data.leak();

        // Transmute back to arena lifetime
        PersonList {
            names: unsafe { std::mem::transmute(names) },
            ages: unsafe { std::mem::transmute(ages) },
            data: unsafe { std::mem::transmute(data) },
        }
    }
}

#[test]
fn test_repeated_fields_basic() {
    // Create builder with static data for encoding
    let builder = PersonListBuilder {
        names: vec!["Alice", "Bob", "Charlie"],
        ages: vec![30, 25, 35],
        data: vec![&[1, 2, 3], &[4, 5], &[6]],
    };

    // Encode
    let encoded = builder.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    // Create arena for decoding
    let arena = Arena::new();

    // Decode into builder
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    // Convert to final struct
    let person_list = decoded_builder.freeze();

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
    // Create empty builder
    let builder = PersonListBuilder::default();

    // Encode (empty builder)
    let encoded = builder.encode_to_vec();

    // Create arena for decoding
    let arena = Arena::new();

    // Decode
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.freeze();

    // Verify all empty
    assert_eq!(person_list.names.len(), 0);
    assert_eq!(person_list.ages.len(), 0);
    assert_eq!(person_list.data.len(), 0);
}

#[test]
fn test_repeated_fields_single_element() {
    // Create builder with one element
    let builder = PersonListBuilder {
        names: vec!["Solo"],
        ages: vec![42],
        data: vec![&[99]],
    };

    let encoded = builder.encode_to_vec();

    // Create arena for decoding
    let arena = Arena::new();
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.freeze();

    assert_eq!(person_list.names.len(), 1);
    assert_eq!(person_list.names[0], "Solo");
    assert_eq!(person_list.ages[0], 42);
    assert_eq!(person_list.data[0], &[99]);
}

#[test]
fn test_repeated_fields_large() {
    // Create a large list with leaked strings
    let mut names = Vec::new();
    let mut ages = Vec::new();
    for i in 0..1000 {
        let name_str: &'static str = Box::leak(format!("Person{}", i).into_boxed_str());
        names.push(name_str);
        ages.push(i);
    }

    let builder = PersonListBuilder {
        names,
        ages,
        data: vec![],
    };

    let encoded = builder.encode_to_vec();
    println!("Encoded {} bytes for 1000 items", encoded.len());

    // Create arena for decoding
    let arena = Arena::new();
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.freeze();

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
    // Create builder with Unicode names
    let builder = PersonListBuilder {
        names: vec!["田中", "José", "Müller", "Владимир"],
        ages: vec![30, 25, 35, 40],
        data: vec![],
    };

    let encoded = builder.encode_to_vec();

    // Create arena for decoding
    let arena = Arena::new();
    let decoded_builder = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let person_list = decoded_builder.freeze();

    assert_eq!(person_list.names[0], "田中");
    assert_eq!(person_list.names[1], "José");
    assert_eq!(person_list.names[2], "Müller");
    assert_eq!(person_list.names[3], "Владимир");
}
