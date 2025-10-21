//! Test for arena-allocated nested messages
//!
//! This test demonstrates nested protobuf messages with arena allocation.
//! The corresponding proto would be:
//!
//! ```proto
//! message Address {
//!   string street = 1;
//!   string city = 2;
//!   int32 zip = 3;
//! }
//!
//! message Person {
//!   string name = 1;
//!   Address address = 2;
//! }
//!
//! message Company {
//!   string name = 1;
//!   repeated Address locations = 2;
//! }
//! ```

use prost::{Arena, DecodeError, Message};
use prost::encoding::{DecodeContext, WireType, string, int32, message};
use bytes::{Buf, BufMut};

/// Address message with arena-allocated string fields
#[derive(Debug, Clone)]
struct Address<'arena> {
    street: &'arena str,
    city: &'arena str,
    zip: i32,
}

impl<'arena> Default for Address<'arena> {
    fn default() -> Self {
        Address {
            street: "",
            city: "",
            zip: 0,
        }
    }
}

impl<'arena> Message<'arena> for Address<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Address {
            street: "",
            city: "",
            zip: 0,
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.street.is_empty() {
            string::encode(1, &self.street.to_string(), buf);
        }
        if !self.city.is_empty() {
            string::encode(2, &self.city.to_string(), buf);
        }
        if self.zip != 0 {
            int32::encode(3, &self.zip, buf);
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
                self.street = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            2 => {
                self.city = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            3 => {
                int32::merge(wire_type, &mut self.zip, buf, ctx)
            }
            _ => {
                prost::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.street.is_empty() {
            len += string::encoded_len(1, &self.street.to_string());
        }
        if !self.city.is_empty() {
            len += string::encoded_len(2, &self.city.to_string());
        }
        if self.zip != 0 {
            len += int32::encoded_len(3, &self.zip);
        }
        len
    }

    fn clear(&mut self) {
        self.street = "";
        self.city = "";
        self.zip = 0;
    }
}

/// Person message with nested Address
#[derive(Debug)]
struct Person<'arena> {
    name: &'arena str,
    address: Option<&'arena Address<'arena>>,
}

impl<'arena> Default for Person<'arena> {
    fn default() -> Self {
        Person {
            name: "",
            address: None,
        }
    }
}

impl<'arena> Message<'arena> for Person<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Person {
            name: "",
            address: None,
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.name.is_empty() {
            string::encode(1, &self.name.to_string(), buf);
        }
        if let Some(address) = self.address {
            message::encode(2, address, buf);
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
                self.name = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            2 => {
                // Decode nested message
                let mut address = Address::default();
                message::merge(wire_type, &mut address, buf, arena, ctx)?;
                // Allocate in arena and store reference
                self.address = Some(arena.alloc(address));
                Ok(())
            }
            _ => {
                prost::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.name.is_empty() {
            len += string::encoded_len(1, &self.name.to_string());
        }
        if let Some(address) = self.address {
            len += message::encoded_len(2, address);
        }
        len
    }

    fn clear(&mut self) {
        self.name = "";
        self.address = None;
    }
}

/// Company with repeated nested messages
#[derive(Debug, Default)]
struct CompanyBuilder {
    name: String,
    locations: Vec<Address<'static>>,  // Temporary storage during decode
}

impl<'arena> Message<'arena> for CompanyBuilder {
    fn new_in(_arena: &'arena Arena) -> Self {
        CompanyBuilder {
            name: String::new(),
            locations: Vec::new(),
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.name.is_empty() {
            string::encode(1, &self.name, buf);
        }
        for location in &self.locations {
            message::encode(2, location, buf);
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
                string::merge(wire_type, &mut self.name, buf, ctx)
            }
            2 => {
                // Decode repeated nested messages
                // This is a bit tricky - we need to transmute the lifetime temporarily
                let mut addr: Address<'arena> = Address::default();
                message::merge(wire_type, &mut addr, buf, arena, ctx)?;
                // SAFETY: We immediately convert to arena storage, so lifetime is ok
                let addr_static: Address<'static> = unsafe { std::mem::transmute(addr) };
                self.locations.push(addr_static);
                Ok(())
            }
            _ => {
                prost::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.name.is_empty() {
            len += string::encoded_len(1, &self.name);
        }
        for location in &self.locations {
            len += message::encoded_len(2, location);
        }
        len
    }

    fn clear(&mut self) {
        self.name.clear();
        self.locations.clear();
    }
}

#[derive(Debug)]
struct Company<'arena> {
    name: &'arena str,
    locations: &'arena [&'arena Address<'arena>],
}

impl CompanyBuilder {
    fn into_arena<'arena>(self, arena: &'arena Arena) -> Company<'arena> {
        // Convert the static lifetime addresses to arena references
        // SAFETY: The addresses contain arena-allocated strings, so the lifetime is correct
        let locations_refs: Vec<&Address<'arena>> = unsafe {
            std::mem::transmute::<Vec<Address<'static>>, Vec<Address<'arena>>>(self.locations)
        }
        .iter()
        .map(|addr| {
            let allocated: &mut Address<'arena> = arena.alloc(addr.clone());
            let immutable: &Address<'arena> = allocated;
            immutable
        })
        .collect();

        Company {
            name: arena.alloc_str(&self.name),
            locations: arena.alloc_slice_copy(&locations_refs),
        }
    }
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
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode person");

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
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    assert_eq!(decoded.name, "Bob");
    assert!(decoded.address.is_none());
}

#[test]
fn test_nested_message_empty() {
    let arena = Arena::new();

    // Person with empty address
    let address = Address::default();
    let person = Person {
        name: "Charlie",
        address: Some(&address),
    };

    let encoded = person.encode_to_vec();
    let decoded = Person::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

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

    let builder = CompanyBuilder {
        name: "Acme Corp".to_string(),
        locations: vec![
            Address { street: "100 First St", city: "Boston", zip: 2101 },
            Address { street: "200 Second Ave", city: "New York", zip: 10001 },
            Address { street: "300 Third Blvd", city: "San Francisco", zip: 94102 },
        ],
    };

    let encoded = builder.encode_to_vec();
    println!("Encoded company with {} locations: {} bytes", 3, encoded.len());

    let decoded_builder = CompanyBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let company = decoded_builder.into_arena(&arena);

    assert_eq!(company.name, "Acme Corp");
    assert_eq!(company.locations.len(), 3);

    assert_eq!(company.locations[0].street, "100 First St");
    assert_eq!(company.locations[0].city, "Boston");
    assert_eq!(company.locations[0].zip, 2101);

    assert_eq!(company.locations[1].street, "200 Second Ave");
    assert_eq!(company.locations[1].city, "New York");

    assert_eq!(company.locations[2].city, "San Francisco");
    assert_eq!(company.locations[2].zip, 94102);

    println!("Arena allocated {} bytes for company with nested messages", arena.allocated_bytes());
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
        let decoded = Person::decode(encoded.as_slice(), &arena)
            .expect("Failed to decode");

        assert_eq!(decoded.name, "Deep Nester");
        assert!(decoded.address.is_some());
        assert_eq!(decoded.address.unwrap().city, "Nested City");
    }

    println!("Arena allocated {} bytes after 10 decodes", arena.allocated_bytes());
}
