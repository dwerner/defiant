//! Tests for skipping the default Debug implementation.

include!(concat!(env!("OUT_DIR"), "/custom_debug.rs"));

use alloc::format;
use core::fmt;

impl<'arena> fmt::Debug for Msg<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Msg {..}")
    }
}

/// A special case with a tuple struct
#[test]
fn tuple_struct_custom_debug() {
    #[derive(Clone, Copy, PartialEq, defiant::View)]
    #[defiant(skip_debug)]
    struct NewType(#[defiant(enumeration = "AnEnum", tag = "5")] i32);
    impl fmt::Debug for NewType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("NewType(custom_debug)")
        }
    }
    assert_eq!(
        format!("{:?}", NewType(AnEnum::B as i32)),
        "NewType(custom_debug)"
    );
    assert_eq!(format!("{:?}", NewType(42)), "NewType(custom_debug)");
}

#[derive(Clone, PartialEq, defiant::Oneof)]
#[defiant(skip_debug)]
pub enum OneofWithEnumCustomDebug<'arena> {
    #[defiant(int32, tag = "8")]
    Int(i32),
    #[defiant(string, tag = "9")]
    String(&'arena str),
    #[defiant(enumeration = "BasicEnumeration", tag = "10")]
    Enumeration(i32),
}
impl<'arena> fmt::Debug for OneofWithEnumCustomDebug<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OneofWithEnumCustomDebug {..}")
    }
}

#[derive(Clone, PartialEq, defiant::View)]
#[defiant(skip_debug)]
struct MessageWithOneofCustomDebug<'arena> {
    #[defiant(oneof = "OneofWithEnumCustomDebug", tags = "8, 9, 10")]
    of: Option<OneofWithEnumCustomDebug<'arena>>,
}

impl<'arena> fmt::Debug for MessageWithOneofCustomDebug<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MessageWithOneofCustomDebug {..}")
    }
}

/// Enumerations inside oneofs
#[test]
fn oneof_with_enum_custom_debug() {
    use defiant::Arena;

    let arena = Arena::new();
    let of = OneofWithEnumCustomDebug::Enumeration(AnEnum::B as i32);
    assert_eq!(format!("{of:?}"), "OneofWithEnumCustomDebug {..}");

    let mut msg_builder = MessageWithOneofCustomDebugBuilder::new_in(&arena);
    msg_builder.set_of(Some(of));
    let msg = msg_builder.freeze();

    assert_eq!(format!("{msg:?}"), "MessageWithOneofCustomDebug {..}");
}

/// Generated protobufs
#[test]
fn test_proto_msg_custom_debug() {
    use defiant::Arena;

    let arena = Arena::new();
    let mut msg_builder = MsgBuilder::new_in(&arena);
    msg_builder.set_a(0);
    msg_builder.set_b("");
    msg_builder.set_c(Some(msg::C::D(AnEnum::A as i32)));
    let msg = msg_builder.freeze();

    assert_eq!(format!("{msg:?}"), "Msg {..}");
}
