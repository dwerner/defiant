//! Test advanced field types with arena allocation

use defiant::{Arena, Encode};
use defiant_derive::View;

#[derive(View)]
struct Data<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(bytes, tag = "2")]
    payload: &'arena [u8],

    #[defiant(bytes, repeated, tag = "3")]
    chunks: &'arena [&'arena [u8]],
}

#[test]
fn test_bytes_field() {
    let arena = Arena::new();
    let mut msg = DataBuilder::new_in(&arena);

    msg.set_name("test");
    msg.set_payload(&[1, 2, 3, 4]);

    assert_eq!(msg.name(), "test");
    assert_eq!(msg.payload(), &[1, 2, 3, 4]);
}

#[test]
fn test_repeated_bytes() {
    let arena = Arena::new();
    let mut msg = DataBuilder::new_in(&arena);

    msg.set_name("test");
    msg.push_chunks(&[1, 2]);
    msg.push_chunks(&[3, 4]);

    assert_eq!(msg.chunks(), &[&[1, 2][..], &[3, 4][..]]);
}

#[test]
fn test_bytes_encode_decode() {
    let arena1 = Arena::new();
    let mut msg = DataBuilder::new_in(&arena1);

    msg.set_name("data");
    msg.set_payload(&[0xDE, 0xAD, 0xBE, 0xEF]);
    msg.push_chunks(&[1, 2, 3]);
    msg.push_chunks(&[4, 5, 6]);

    let view = msg.freeze();

    // Encode
    let mut buf = Vec::new();
    view.encode(&mut buf).unwrap();

    // Decode
    let arena2 = Arena::new();
    let decoded = DataBuilder::decode(&buf[..], &arena2).unwrap();

    assert_eq!(decoded.name, "data");
    assert_eq!(decoded.payload, &[0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(decoded.chunks.len(), 2);
    assert_eq!(decoded.chunks[0], &[1, 2, 3]);
    assert_eq!(decoded.chunks[1], &[4, 5, 6]);
}
