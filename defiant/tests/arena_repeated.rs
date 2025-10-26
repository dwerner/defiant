//! Test for arena-allocated repeated fields

use defiant::{Arena, Message, Encode};

/// PersonList with repeated fields
#[derive(Message)]
struct PersonList<'arena> {
    #[defiant(string, repeated, tag = 1)]
    names: &'arena [&'arena str],
    #[defiant(int32, repeated, tag = 2)]
    ages: &'arena [i32],
    #[defiant(bytes, repeated, tag = 3)]
    data: &'arena [&'arena [u8]],
}

#[test]
fn test_repeated_fields_basic() {
    let arena = Arena::new();

    // Create list with data
    let list = PersonList {
        names: &["Alice", "Bob", "Charlie"],
        ages: &[30, 25, 35],
        data: &[&[1, 2, 3], &[4, 5], &[6]],
    };

    // Encode
    let encoded = list.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    // Decode
    let decoded = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    // Verify
    assert_eq!(decoded.names.len(), 3);
    assert_eq!(decoded.names[0], "Alice");
    assert_eq!(decoded.names[1], "Bob");
    assert_eq!(decoded.names[2], "Charlie");

    assert_eq!(decoded.ages.len(), 3);
    assert_eq!(decoded.ages, &[30, 25, 35]);

    assert_eq!(decoded.data.len(), 3);
    assert_eq!(decoded.data[0], &[1, 2, 3]);
    assert_eq!(decoded.data[1], &[4, 5]);
    assert_eq!(decoded.data[2], &[6]);

    println!("Successfully decoded repeated fields");
    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_repeated_fields_empty() {
    let arena = Arena::new();

    // Create empty list
    let list = PersonList {
        names: &[],
        ages: &[],
        data: &[],
    };

    // Encode
    let encoded = list.encode_to_vec();

    // Decode
    let decoded = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    // Verify all empty
    assert_eq!(decoded.names.len(), 0);
    assert_eq!(decoded.ages.len(), 0);
    assert_eq!(decoded.data.len(), 0);
}

#[test]
fn test_repeated_fields_single_element() {
    let arena = Arena::new();

    // Create list with one element
    let list = PersonList {
        names: &["Solo"],
        ages: &[42],
        data: &[&[99]],
    };

    let encoded = list.encode_to_vec();

    let decoded = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    assert_eq!(decoded.names.len(), 1);
    assert_eq!(decoded.names[0], "Solo");
    assert_eq!(decoded.ages[0], 42);
    assert_eq!(decoded.data[0], &[99]);
}

#[test]
fn test_repeated_fields_large() {
    let arena = Arena::new();

    // Create a large list
    let mut names_vec = Vec::new();
    let mut ages_vec = Vec::new();
    for i in 0..1000 {
        let name_str: &'static str = Box::leak(format!("Person{}", i).into_boxed_str());
        names_vec.push(name_str);
        ages_vec.push(i);
    }

    let list = PersonList {
        names: names_vec.leak(),
        ages: ages_vec.leak(),
        data: &[],
    };

    let encoded = list.encode_to_vec();
    println!("Encoded {} bytes for 1000 items", encoded.len());

    let decoded = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    assert_eq!(decoded.names.len(), 1000);
    assert_eq!(decoded.ages.len(), 1000);

    // Verify some samples
    assert_eq!(decoded.names[0], "Person0");
    assert_eq!(decoded.names[500], "Person500");
    assert_eq!(decoded.names[999], "Person999");

    assert_eq!(decoded.ages[0], 0);
    assert_eq!(decoded.ages[500], 500);
    assert_eq!(decoded.ages[999], 999);

    println!("Arena allocated {} bytes for 1000 items", arena.allocated_bytes());
}

#[test]
fn test_repeated_unicode() {
    let arena = Arena::new();

    // Create list with Unicode names
    let list = PersonList {
        names: &["田中", "José", "Müller", "Владимир"],
        ages: &[30, 25, 35, 40],
        data: &[],
    };

    let encoded = list.encode_to_vec();

    let decoded = PersonListBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    assert_eq!(decoded.names[0], "田中");
    assert_eq!(decoded.names[1], "José");
    assert_eq!(decoded.names[2], "Müller");
    assert_eq!(decoded.names[3], "Владимир");
}
