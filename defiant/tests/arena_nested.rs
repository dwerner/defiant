//! Test for arena-allocated nested messages
//!
//! This test demonstrates nested protobuf messages with arena allocation.

use defiant_derive::View;
use defiant::{Arena, Encode};

/// Address message with arena-allocated string fields
#[derive(Clone, PartialEq, Debug, View)]
#[defiant(skip_debug)]
struct Address<'arena> {
    #[defiant(string, tag = 1)]
    street: &'arena str,
    #[defiant(string, tag = 2)]
    city: &'arena str,
    #[defiant(int32, tag = 3)]
    zip: i32,
}

/// Person message with nested Address
#[derive(View)]
struct Person<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(message, tag = 2)]
    address: Option<&'arena Address<'arena>>,
}

/// Company with repeated nested messages
#[derive(View)]
struct Company<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(message, repeated, tag = 2)]
    locations: &'arena [&'arena Address<'arena>],
}

#[test]
fn test_nested_message_basic() {
    let arena = Arena::new();

    // Create a person with address
    let address = Address {
        street: "123 Main St",
        city: "Springfield",
        zip: 12345,
    };

    let person = Person {
        name: "Alice",
        address: Some(&address),
    };

    // Encode
    let encoded = person.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    // Decode
    let decoded = PersonBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person")
        .freeze();

    // Verify
    assert_eq!(decoded.name, "Alice");
    assert!(decoded.address.is_some());

    let decoded_address = decoded.address.unwrap();
    assert_eq!(decoded_address.street, "123 Main St");
    assert_eq!(decoded_address.city, "Springfield");
    assert_eq!(decoded_address.zip, 12345);

    println!("Successfully decoded nested message");
    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_nested_message_none() {
    let arena = Arena::new();

    // Person without address
    let person = Person {
        name: "Bob",
        address: None,
    };

    let encoded = person.encode_to_vec();
    let decoded = PersonBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.name, "Bob");
    assert!(decoded.address.is_none());
}

#[test]
fn test_nested_message_empty() {
    let arena = Arena::new();

    // Person with empty address
    let address = Address {
        street: "",
        city: "",
        zip: 0,
    };
    let person = Person {
        name: "Charlie",
        address: Some(&address),
    };

    let encoded = person.encode_to_vec();
    let decoded = PersonBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.name, "Charlie");
    assert!(decoded.address.is_some());

    let decoded_address = decoded.address.unwrap();
    assert_eq!(decoded_address.street, "");
    assert_eq!(decoded_address.city, "");
    assert_eq!(decoded_address.zip, 0);
}

#[test]
fn test_repeated_nested_messages() {
    let arena = Arena::new();

    let addr1 = Address {
        street: "100 First St",
        city: "Boston",
        zip: 2101,
    };
    let addr2 = Address {
        street: "200 Second Ave",
        city: "New York",
        zip: 10001,
    };
    let addr3 = Address {
        street: "300 Third Blvd",
        city: "San Francisco",
        zip: 94102,
    };

    let company = Company {
        name: "Acme Corp",
        locations: &[&addr1, &addr2, &addr3],
    };

    let encoded = company.encode_to_vec();
    println!(
        "Encoded company with {} locations: {} bytes",
        3,
        encoded.len()
    );

    let decoded = CompanyBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.name, "Acme Corp");
    assert_eq!(decoded.locations.len(), 3);

    assert_eq!(decoded.locations[0].street, "100 First St");
    assert_eq!(decoded.locations[0].city, "Boston");
    assert_eq!(decoded.locations[0].zip, 2101);

    assert_eq!(decoded.locations[1].street, "200 Second Ave");
    assert_eq!(decoded.locations[1].city, "New York");

    assert_eq!(decoded.locations[2].city, "San Francisco");
    assert_eq!(decoded.locations[2].zip, 94102);

    println!(
        "Arena allocated {} bytes for company with nested messages",
        arena.allocated_bytes()
    );
}

#[test]
fn test_deeply_nested() {
    let arena = Arena::new();

    // Create nested structure
    let address = Address {
        street: "Deep Street",
        city: "Nested City",
        zip: 99999,
    };

    let person = Person {
        name: "Deep Nester",
        address: Some(&address),
    };

    // Encode and decode multiple times to test arena allocation
    let encoded = person.encode_to_vec();

    for _ in 0..10 {
        let decoded = PersonBuilder::decode(encoded.as_slice(), &arena)
            .expect("Failed to decode")
            .freeze();

        assert_eq!(decoded.name, "Deep Nester");
        assert!(decoded.address.is_some());
        assert_eq!(decoded.address.unwrap().city, "Nested City");
    }

    println!(
        "Arena allocated {} bytes after 10 decodes",
        arena.allocated_bytes()
    );
}
