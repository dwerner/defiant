//! Test nested message support

use prost::Arena;
use prost_derive::Message;

#[derive(Message)]
struct Address<'arena> {
    #[prost(string, tag = "1")]
    street: &'arena str,

    #[prost(string, tag = "2")]
    city: &'arena str,

    #[prost(int32, tag = "3")]
    zip: i32,
}

// Test simple message without nested messages first
#[derive(Message)]
struct SimplePerson<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(int32, tag = "2")]
    age: i32,
}

#[derive(Message)]
struct Person<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(message, tag = "2")]
    address: Option<&'arena Address<'arena>>,
}

#[test]
fn test_optional_message() {
    let arena = Arena::new();

    // Create nested message
    let mut addr = AddressMessage::new_in(&arena);
    addr.set_street("123 Main St");
    addr.set_city("Springfield");
    addr.set_zip(12345);
    let address_view = addr.into_view();

    // Create parent with nested message
    let mut person = PersonMessage::new_in(&arena);
    person.set_name("Alice");
    person.set_address(Some(address_view));

    assert_eq!(person.name(), "Alice");
    assert!(person.address().is_some());
    assert_eq!(person.address().unwrap().street, "123 Main St");
}

#[derive(Message)]
struct PersonWithRepeated<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(message, repeated, tag = "2")]
    previous_addresses: &'arena [Address<'arena>],
}

#[test]
fn test_repeated_messages() {
    let arena = Arena::new();

    let mut addr1 = AddressMessage::new_in(&arena);
    addr1.set_street("Old Street");
    addr1.set_city("OldCity");
    addr1.set_zip(11111);

    let mut addr2 = AddressMessage::new_in(&arena);
    addr2.set_street("Older Street");
    addr2.set_city("OlderCity");
    addr2.set_zip(22222);

    let mut person = PersonWithRepeatedMessage::new_in(&arena);
    person.set_name("Bob");
    person.push_previous_addresses(addr1.into_view());
    person.push_previous_addresses(addr2.into_view());

    assert_eq!(person.previous_addresses().len(), 2);
    assert_eq!(person.previous_addresses()[0].street, "Old Street");
}
