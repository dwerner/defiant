//! Test nested message support

use defiant::Arena;
use defiant_derive::View;

#[derive(Clone, View)]
struct Address<'arena> {
    #[defiant(string, tag = "1")]
    street: &'arena str,

    #[defiant(string, tag = "2")]
    city: &'arena str,

    #[defiant(int32, tag = "3")]
    zip: i32,
}

// Test simple message without nested messages first
#[derive(View)]
struct SimplePerson<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(int32, tag = "2")]
    age: i32,
}

#[derive(View)]
struct Person<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(message, tag = "2")]
    address: Option<&'arena Address<'arena>>,
}

#[test]
fn test_optional_message() {
    let arena = Arena::new();

    // Create nested message
    let mut addr = AddressBuilder::new_in(&arena);
    addr.set_street("123 Main St");
    addr.set_city("Springfield");
    addr.set_zip(12345);
    let address_view = addr.freeze();

    // Create parent with nested message
    let mut person = PersonBuilder::new_in(&arena);
    person.set_name("Alice");
    person.set_address(Some(&address_view));

    assert_eq!(person.name(), "Alice");
    assert!(person.address().is_some());
    assert_eq!(person.address().unwrap().street, "123 Main St");
}

#[derive(View)]
struct PersonWithRepeated<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(message, repeated, tag = "2")]
    previous_addresses: &'arena [Address<'arena>],
}

#[test]
fn test_repeated_messages() {
    let arena = Arena::new();

    let mut addr1 = AddressBuilder::new_in(&arena);
    addr1.set_street("Old Street");
    addr1.set_city("OldCity");
    addr1.set_zip(11111);

    let mut addr2 = AddressBuilder::new_in(&arena);
    addr2.set_street("Older Street");
    addr2.set_city("OlderCity");
    addr2.set_zip(22222);

    let mut person = PersonWithRepeatedBuilder::new_in(&arena);
    person.set_name("Bob");
    person.push_previous_addresses(addr1.freeze());
    person.push_previous_addresses(addr2.freeze());

    assert_eq!(person.previous_addresses().len(), 2);
    assert_eq!(person.previous_addresses()[0].street, "Old Street");
}
