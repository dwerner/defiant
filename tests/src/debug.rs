#![cfg(ignore)]
// TODO: Migrate to View API
//! Tests for our own Debug implementation.
//!
//! The tests check against expected output. This may be a bit fragile, but it is likely OK for
//! actual use.

use defiant::alloc::format;
#[cfg(not(feature = "std"))]
use defiant::alloc::string::String;

// Borrow some types from other places.
#[cfg(feature = "std")]
use crate::message_encoding::Basic;
use crate::message_encoding::BasicEnumeration;

/// Some real-life message
#[test]
#[cfg(feature = "std")]
fn basic() {
    let mut basic = Basic::default();
    assert_eq!(
        format!("{basic:?}"),
        "Basic { \
         int32: 0, \
         bools: [], \
         string: \"\", \
         optional_string: None, \
         enumeration: ZERO, \
         enumeration_map: {}, \
         string_map: {}, \
         enumeration_btree_map: {}, \
         string_btree_map: {}, \
         oneof: None, \
         bytes_map: {} \
         }"
    );
    basic
        .enumeration_map
        .insert(0, BasicEnumeration::TWO as i32);
    basic.enumeration = 42;
    basic
        .bytes_map
        .insert("hello".to_string(), "world".as_bytes().into());
    assert_eq!(
        format!("{basic:?}"),
        "Basic { \
         int32: 0, \
         bools: [], \
         string: \"\", \
         optional_string: None, \
         enumeration: 42, \
         enumeration_map: {0: TWO}, \
         string_map: {}, \
         enumeration_btree_map: {}, \
         string_btree_map: {}, \
         oneof: None, \
         bytes_map: {\"hello\": [119, 111, 114, 108, 100]} \
         }"
    );
}

/// A special case with a tuple struct
#[test]
fn tuple_struct() {
    #[derive(Clone, PartialEq, defiant::View)]
    struct NewType(#[defiant(enumeration = "BasicEnumeration", tag = "5")] i32);
    assert_eq!(
        format!("{:?}", NewType(BasicEnumeration::TWO as i32)),
        "NewType(TWO)"
    );
    assert_eq!(format!("{:?}", NewType(42)), "NewType(42)");
}

#[derive(Clone, PartialEq, defiant::Oneof)]
pub enum OneofWithEnum<'arena> {
    #[defiant(int32, tag = "8")]
    Int(i32),
    #[defiant(string, tag = "9")]
    String(&'arena str),
    #[defiant(enumeration = "BasicEnumeration", tag = "10")]
    Enumeration(i32),
}

#[derive(Clone, PartialEq, defiant::View)]
struct MessageWithOneof<'arena> {
    #[defiant(oneof = "OneofWithEnum", tags = "8, 9, 10")]
    of: Option<OneofWithEnum<'arena>>,
}

/// Enumerations inside oneofs
#[test]
fn oneof_with_enum() {
    use defiant::Arena;

    let arena = Arena::new();
    let mut msg_builder = MessageWithOneofBuilder::new_in(&arena);
    msg_builder.set_of(Some(OneofWithEnum::Enumeration(BasicEnumeration::TWO as i32)));
    let msg = msg_builder.freeze();

    assert_eq!(
        format!("{msg:?}"),
        "MessageWithOneof { of: Some(Enumeration(TWO)) }"
    );
}
