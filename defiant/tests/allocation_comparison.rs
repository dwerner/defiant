//! Test that arena-based messages decode correctly

use defiant::{Arena, Message};

#[derive(Debug)]
struct PersonArena<'arena> {
    name: &'arena str,
    email: &'arena str,
    phone: &'arena str,
    address: &'arena str,
}

impl<'arena> Default for PersonArena<'arena> {
    fn default() -> Self {
        PersonArena {
            name: "",
            email: "",
            phone: "",
            address: "",
        }
    }
}

impl<'arena> Message<'arena> for PersonArena<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Self::default()
    }

    fn encode_raw(&self, buf: &mut impl defiant::bytes::BufMut) {
        if !self.name.is_empty() {
            defiant::encoding::string::encode(1, self.name, buf);
        }
        if !self.email.is_empty() {
            defiant::encoding::string::encode(2, self.email, buf);
        }
        if !self.phone.is_empty() {
            defiant::encoding::string::encode(3, self.phone, buf);
        }
        if !self.address.is_empty() {
            defiant::encoding::string::encode(4, self.address, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: defiant::encoding::wire_type::WireType,
        buf: &mut impl defiant::bytes::Buf,
        arena: &'arena Arena,
        ctx: defiant::encoding::DecodeContext,
    ) -> Result<(), defiant::DecodeError> {
        match tag {
            1 => defiant::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.name = v),
            2 => defiant::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.email = v),
            3 => defiant::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.phone = v),
            4 => defiant::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.address = v),
            _ => defiant::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0
            + if !self.name.is_empty() {
                defiant::encoding::string::encoded_len(1, self.name)
            } else {
                0
            }
            + if !self.email.is_empty() {
                defiant::encoding::string::encoded_len(2, self.email)
            } else {
                0
            }
            + if !self.phone.is_empty() {
                defiant::encoding::string::encoded_len(3, self.phone)
            } else {
                0
            }
            + if !self.address.is_empty() {
                defiant::encoding::string::encoded_len(4, self.address)
            } else {
                0
            }
    }

}

fn create_test_data() -> Vec<u8> {
    let mut data = Vec::new();
    // name = "Alice Johnson" (13 bytes)
    data.extend_from_slice(&[0x0a, 0x0d]);
    data.extend_from_slice(b"Alice Johnson");
    // email = "alice.johnson@example.com" (25 bytes)
    data.extend_from_slice(&[0x12, 0x19]);
    data.extend_from_slice(b"alice.johnson@example.com");
    // phone = "+1-555-0123" (11 bytes)
    data.extend_from_slice(&[0x1a, 0x0b]);
    data.extend_from_slice(b"+1-555-0123");
    // address = "123 Main Street, Portland, OR 97201" (35 bytes)
    data.extend_from_slice(&[0x22, 0x23]);
    data.extend_from_slice(b"123 Main Street, Portland, OR 97201");
    data
}

#[test]
fn test_decode() {
    let data = create_test_data();

    // Verify arena-based &str approach decodes correctly
    for _ in 0..10 {
        let arena = Arena::new();
        let msg = PersonArena::decode(&data[..], &arena).unwrap();
        assert_eq!(msg.name, "Alice Johnson");
        assert_eq!(msg.email, "alice.johnson@example.com");
        assert_eq!(msg.phone, "+1-555-0123");
        assert_eq!(msg.address, "123 Main Street, Portland, OR 97201");
    }
}

