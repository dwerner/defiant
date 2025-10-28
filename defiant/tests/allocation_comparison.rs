//! Test that arena-based messages decode correctly

use defiant::Arena;

#[derive(Clone, PartialEq, defiant::View)]
struct PersonArena<'arena> {
    #[defiant(string, required, tag = "1")]
    name: &'arena str,
    #[defiant(string, required, tag = "2")]
    email: &'arena str,
    #[defiant(string, required, tag = "3")]
    phone: &'arena str,
    #[defiant(string, required, tag = "4")]
    address: &'arena str,
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
        let msg = PersonArena::from_buf(&data[..], &arena).unwrap();
        assert_eq!(msg.name, "Alice Johnson");
        assert_eq!(msg.email, "alice.johnson@example.com");
        assert_eq!(msg.phone, "+1-555-0123");
        assert_eq!(msg.address, "123 Main Street, Portland, OR 97201");
    }
}
