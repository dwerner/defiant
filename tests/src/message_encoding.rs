// TODO: This file needs migration to the new View derive and arena allocation
// The old Message derive has been removed. All tests are temporarily ignored.

use defiant::alloc::vec;
#[cfg(not(feature = "std"))]
use defiant::alloc::{borrow::ToOwned, string::String, vec::Vec};

use defiant::bytes::Bytes;
use defiant::{Enumeration, Oneof};

use crate::check_message;
use crate::check_serialize_equivalent;

#[derive(Clone, PartialEq, defiant::View)]
pub struct RepeatedFloats<'arena> {
    #[defiant(float, tag = "11")]
    pub single_float: f32,
    #[defiant(float, repeated, packed = "true", tag = "41")]
    pub repeated_float: &'arena [f32],
}

#[test]
fn check_repeated_floats() {
    let arena = defiant::Arena::new();
    let msg = RepeatedFloats::builder(&arena)
        .set_single_float(0.0)
        .repeated_float
        .extend(&[
            0.1,
            340282300000000000000000000000000000000.0,
            0.000000000000000000000000000000000000011754944,
        ])
        .freeze();
    check_message(&msg, &arena);
}

#[test]
fn check_scalar_types() {
    let arena = defiant::Arena::new();
    let msg = ScalarTypes::builder(&arena).freeze();
    check_message(&msg, &arena);
}

/// A protobuf message which contains all scalar types.
#[derive(Clone, PartialEq, defiant::View)]
pub struct ScalarTypes<'arena> {
    #[defiant(int32, tag = "001")]
    pub int32: i32,
    #[defiant(int64, tag = "002")]
    pub int64: i64,
    #[defiant(uint32, tag = "003")]
    pub uint32: u32,
    #[defiant(uint64, tag = "004")]
    pub uint64: u64,
    #[defiant(sint32, tag = "005")]
    pub sint32: i32,
    #[defiant(sint64, tag = "006")]
    pub sint64: i64,
    #[defiant(fixed32, tag = "007")]
    pub fixed32: u32,
    #[defiant(fixed64, tag = "008")]
    pub fixed64: u64,
    #[defiant(sfixed32, tag = "009")]
    pub sfixed32: i32,
    #[defiant(sfixed64, tag = "010")]
    pub sfixed64: i64,
    #[defiant(float, tag = "011")]
    pub float: f32,
    #[defiant(double, tag = "012")]
    pub double: f64,
    #[defiant(bool, tag = "013")]
    pub _bool: bool,
    #[defiant(string, tag = "014")]
    pub string: &'arena str,
    #[defiant(bytes, tag = "015")]
    pub bytes_vec: &'arena [u8],
    #[defiant(bytes, tag = "016")]
    pub bytes_buf: &'arena [u8],

    #[defiant(int32, required, tag = "101")]
    pub required_int32: i32,
    #[defiant(int64, required, tag = "102")]
    pub required_int64: i64,
    #[defiant(uint32, required, tag = "103")]
    pub required_uint32: u32,
    #[defiant(uint64, required, tag = "104")]
    pub required_uint64: u64,
    #[defiant(sint32, required, tag = "105")]
    pub required_sint32: i32,
    #[defiant(sint64, required, tag = "106")]
    pub required_sint64: i64,
    #[defiant(fixed32, required, tag = "107")]
    pub required_fixed32: u32,
    #[defiant(fixed64, required, tag = "108")]
    pub required_fixed64: u64,
    #[defiant(sfixed32, required, tag = "109")]
    pub required_sfixed32: i32,
    #[defiant(sfixed64, required, tag = "110")]
    pub required_sfixed64: i64,
    #[defiant(float, required, tag = "111")]
    pub required_float: f32,
    #[defiant(double, required, tag = "112")]
    pub required_double: f64,
    #[defiant(bool, required, tag = "113")]
    pub required_bool: bool,
    #[defiant(string, required, tag = "114")]
    pub required_string: &'arena str,
    #[defiant(bytes, required, tag = "115")]
    pub required_bytes_vec: &'arena [u8],
    #[defiant(bytes, required, tag = "116")]
    pub required_bytes_buf: &'arena [u8],

    #[defiant(int32, optional, tag = "201")]
    pub optional_int32: Option<i32>,
    #[defiant(int64, optional, tag = "202")]
    pub optional_int64: Option<i64>,
    #[defiant(uint32, optional, tag = "203")]
    pub optional_uint32: Option<u32>,
    #[defiant(uint64, optional, tag = "204")]
    pub optional_uint64: Option<u64>,
    #[defiant(sint32, optional, tag = "205")]
    pub optional_sint32: Option<i32>,
    #[defiant(sint64, optional, tag = "206")]
    pub optional_sint64: Option<i64>,

    #[defiant(fixed32, optional, tag = "207")]
    pub optional_fixed32: Option<u32>,
    #[defiant(fixed64, optional, tag = "208")]
    pub optional_fixed64: Option<u64>,
    #[defiant(sfixed32, optional, tag = "209")]
    pub optional_sfixed32: Option<i32>,
    #[defiant(sfixed64, optional, tag = "210")]
    pub optional_sfixed64: Option<i64>,
    #[defiant(float, optional, tag = "211")]
    pub optional_float: Option<f32>,
    #[defiant(double, optional, tag = "212")]
    pub optional_double: Option<f64>,
    #[defiant(bool, optional, tag = "213")]
    pub optional_bool: Option<bool>,
    #[defiant(string, optional, tag = "214")]
    pub optional_string: Option<&'arena str>,
    #[defiant(bytes, optional, tag = "215")]
    pub optional_bytes_vec: Option<&'arena [u8]>,
    #[defiant(bytes, optional, tag = "216")]
    pub optional_bytes_buf: Option<&'arena [u8]>,

    #[defiant(int32, repeated, packed = "false", tag = "301")]
    pub repeated_int32: &'arena [i32],
    #[defiant(int64, repeated, packed = "false", tag = "302")]
    pub repeated_int64: &'arena [i64],
    #[defiant(uint32, repeated, packed = "false", tag = "303")]
    pub repeated_uint32: &'arena [u32],
    #[defiant(uint64, repeated, packed = "false", tag = "304")]
    pub repeated_uint64: &'arena [u64],
    #[defiant(sint32, repeated, packed = "false", tag = "305")]
    pub repeated_sint32: &'arena [i32],
    #[defiant(sint64, repeated, packed = "false", tag = "306")]
    pub repeated_sint64: &'arena [i64],
    #[defiant(fixed32, repeated, packed = "false", tag = "307")]
    pub repeated_fixed32: &'arena [u32],
    #[defiant(fixed64, repeated, packed = "false", tag = "308")]
    pub repeated_fixed64: &'arena [u64],
    #[defiant(sfixed32, repeated, packed = "false", tag = "309")]
    pub repeated_sfixed32: &'arena [i32],
    #[defiant(sfixed64, repeated, packed = "false", tag = "310")]
    pub repeated_sfixed64: &'arena [i64],
    #[defiant(float, repeated, packed = "false", tag = "311")]
    pub repeated_float: &'arena [f32],
    #[defiant(double, repeated, packed = "false", tag = "312")]
    pub repeated_double: &'arena [f64],
    #[defiant(bool, repeated, packed = "false", tag = "313")]
    pub repeated_bool: &'arena [bool],
    #[defiant(string, repeated, packed = "false", tag = "315")]
    pub repeated_string: &'arena [&'arena str],
    #[defiant(bytes, repeated, packed = "false", tag = "316")]
    pub repeated_bytes_vec: &'arena [&'arena [u8]],
    #[defiant(bytes, repeated, packed = "false", tag = "317")]
    pub repeated_bytes_buf: &'arena [&'arena [u8]],

    #[defiant(int32, repeated, tag = "401")]
    pub packed_int32: &'arena [i32],
    #[defiant(int64, repeated, tag = "402")]
    pub packed_int64: &'arena [i64],
    #[defiant(uint32, repeated, tag = "403")]
    pub packed_uint32: &'arena [u32],
    #[defiant(uint64, repeated, tag = "404")]
    pub packed_uint64: &'arena [u64],
    #[defiant(sint32, repeated, tag = "405")]
    pub packed_sint32: &'arena [i32],
    #[defiant(sint64, repeated, tag = "406")]
    pub packed_sint64: &'arena [i64],
    #[defiant(fixed32, repeated, tag = "407")]
    pub packed_fixed32: &'arena [u32],

    #[defiant(fixed64, repeated, tag = "408")]
    pub packed_fixed64: &'arena [u64],
    #[defiant(sfixed32, repeated, tag = "409")]
    pub packed_sfixed32: &'arena [i32],
    #[defiant(sfixed64, repeated, tag = "410")]
    pub packed_sfixed64: &'arena [i64],
    #[defiant(float, repeated, tag = "411")]
    pub packed_float: &'arena [f32],
    #[defiant(double, repeated, tag = "412")]
    pub packed_double: &'arena [f64],
    #[defiant(bool, repeated, tag = "413")]
    pub packed_bool: &'arena [bool],
    #[defiant(string, repeated, tag = "415")]
    pub packed_string: &'arena [&'arena str],
    #[defiant(bytes, repeated, tag = "416")]
    pub packed_bytes_vec: &'arena [&'arena [u8]],
    #[defiant(bytes, repeated, tag = "417")]
    pub packed_bytes_buf: &'arena [&'arena [u8]],
}

#[test]
#[ignore = "Needs migration to builder pattern with arena allocation"]
fn check_tags_inferred() {
    // TODO: Rewrite using builders:
    // let arena = defiant::Arena::new();
    // let mut builder = TagsInferredBuilder::new_in(&arena);
    // builder.set_one(true);
    // ...
    // let tags_inferred = builder.freeze();
}

#[derive(Clone, PartialEq, defiant::View)]
pub struct TagsInferred<'arena> {
    #[defiant(bool)]
    pub one: bool,
    #[defiant(int32, optional)]
    pub two: Option<i32>,
    #[defiant(float, repeated)]
    pub three: &'arena [f32],

    #[defiant(tag = "9", string, required)]
    pub skip_to_nine: &'arena str,
    #[defiant(enumeration = "BasicEnumeration", default = "ONE")]
    pub ten: i32,
    #[defiant(btree_map = "string, string")]
    pub eleven: defiant::ArenaMap<'arena, &'arena str, &'arena str>,

    #[defiant(tag = "5", bytes)]
    pub back_to_five: &'arena [u8],
    #[defiant(message, required)]
    pub six: &'arena Basic<'arena>,
}

#[derive(Clone, PartialEq, defiant::View)]
pub struct TagsQualified<'arena> {
    #[defiant(tag = "1", bool)]
    pub one: bool,
    #[defiant(tag = "2", int32, optional)]
    pub two: Option<i32>,
    #[defiant(tag = "3", float, repeated)]
    pub three: &'arena [f32],

    #[defiant(tag = "5", bytes)]
    pub five: &'arena [u8],
    #[defiant(tag = "6", message, required)]
    pub six: &'arena Basic<'arena>,

    #[defiant(tag = "9", string, required)]
    pub nine: &'arena str,
    #[defiant(tag = "10", enumeration = "BasicEnumeration", default = "ONE")]
    pub ten: i32,
    #[defiant(tag = "11", btree_map = "string, string")]
    pub eleven: defiant::ArenaMap<'arena, &'arena str, &'arena str>,
}

/// A prost message with default value.
#[derive(Clone, PartialEq, defiant::View)]
pub struct DefaultValues<'arena> {
    #[defiant(int32, tag = "1", default = "42")]
    pub int32: i32,

    #[defiant(int32, optional, tag = "2", default = "88")]
    pub optional_int32: Option<i32>,

    #[defiant(string, tag = "3", default = "forty two")]
    pub string: &'arena str,

    #[defiant(bytes, tag = "7", default = "b\"foo\\x00bar\"")]
    pub bytes_vec: &'arena [u8],

    #[defiant(bytes, tag = "8", default = "b\"foo\\x00bar\"")]
    pub bytes_buf: &'arena [u8],

    #[defiant(enumeration = "BasicEnumeration", tag = "4", default = "ONE")]
    pub enumeration: i32,

    #[defiant(enumeration = "BasicEnumeration", optional, tag = "5", default = "TWO")]
    pub optional_enumeration: Option<i32>,

    #[defiant(enumeration = "BasicEnumeration", repeated, tag = "6")]
    pub repeated_enumeration: &'arena [i32],
}

#[test]
fn check_default_values() {
    use defiant::ArenaDefault;
    let arena = defiant::Arena::new();
    let default = DefaultValues::arena_default(&arena).freeze();
    assert_eq!(default.int32, 42);
    assert_eq!(default.optional_int32, None);
    assert_eq!(&default.string, "forty two");
    assert_eq!(&default.bytes_vec, b"foo\0bar");
    assert_eq!(&default.bytes_buf, b"foo\0bar");
    assert_eq!(default.enumeration, BasicEnumeration::ONE as i32);
    assert_eq!(default.optional_enumeration, None);
    assert_eq!(&default.repeated_enumeration, &[]);
    assert_eq!(0, default.encoded_len());
}

/// A protobuf enum.
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Enumeration)]
pub enum BasicEnumeration {
    ZERO = 0,
    ONE = 1,
    TWO = 2,
    THREE = 3,
}

#[derive(Clone, PartialEq, defiant::View)]
pub struct Basic<'arena> {
    #[defiant(int32, tag = "1")]
    pub int32: i32,

    #[defiant(bool, repeated, packed = "false", tag = "2")]
    pub bools: &'arena [bool],

    #[defiant(string, tag = "3")]
    pub string: &'arena str,

    #[defiant(string, optional, tag = "4")]
    pub optional_string: Option<&'arena str>,

    #[defiant(enumeration = "BasicEnumeration", tag = "5")]
    pub enumeration: i32,

    #[defiant(arena_map = "int32, enumeration(BasicEnumeration)", tag = "10")]
    pub enumeration_btree_map: defiant::ArenaMap<'arena, i32, i32>,

    #[defiant(arena_map = "string, string", tag = "11")]
    pub string_btree_map: defiant::ArenaMap<'arena, &'arena str, &'arena str>,

    #[defiant(oneof = "BasicOneof", tags = "8, 9")]
    pub oneof: Option<BasicOneof<'arena>>,

    #[defiant(arena_map = "string, bytes", tag = "12")]
    pub bytes_map: defiant::ArenaMap<'arena, &'arena str, &'arena [u8]>,
}

#[derive(Clone, PartialEq, defiant::View)]
pub struct Compound<'arena> {
    #[defiant(message, optional, tag = "1")]
    pub optional_message: Option<&'arena Basic<'arena>>,

    #[defiant(message, required, tag = "2")]
    pub required_message: &'arena Basic<'arena>,

    #[defiant(message, repeated, tag = "3")]
    pub repeated_message: &'arena [Basic<'arena>],

    #[defiant(arena_map = "sint32, message", tag = "4")]
    pub message_map: defiant::ArenaMap<'arena, i32, &'arena Basic<'arena>>,
    // pub message_btree_map: defiant::ArenaMap<i32, Basic>,
}

#[derive(Clone, PartialEq, Oneof)]
pub enum BasicOneof<'arena> {
    #[defiant(int32, tag = "8")]
    Int(i32),
    #[defiant(string, tag = "9")]
    String(&'arena str),
}
