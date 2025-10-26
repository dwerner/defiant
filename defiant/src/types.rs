//! Protocol Buffers well-known wrapper types.
//!
//! This module provides implementations of `Message` for Rust standard library types which
//! correspond to a Protobuf well-known wrapper type. The remaining well-known types are defined in
//! the `prost-types` crate in order to avoid a cyclic dependency between `prost` and
//! `prost-build`.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ::bytes::{Buf, BufMut, Bytes};

use crate::arena::Arena;
use crate::encoding::wire_type::WireType;
use crate::{
    encoding::{
        bool, bytes, double, float, int32, int64, skip_field, string, uint32, uint64, DecodeContext,
    },
    Decode, DecodeError, Encode, Name,
};

/// `google.protobuf.BoolValue`
impl Encode for bool {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self {
            bool::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self {
            2
        } else {
            0
        }
    }
}

impl Decode<'_> for bool {
    fn new_in(_arena: &Arena) -> Self {
        false
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            bool::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.BoolValue`
impl Name for bool {
    const NAME: &'static str = "BoolValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt32Value`
impl Encode for u32 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint32::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            uint32::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for u32 {
    fn new_in(_arena: &Arena) -> Self {
        0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            uint32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.UInt32Value`
impl Name for u32 {
    const NAME: &'static str = "UInt32Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt64Value`
impl Encode for u64 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint64::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            uint64::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for u64 {
    fn new_in(_arena: &Arena) -> Self {
        0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            uint64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.UInt64Value`
impl Name for u64 {
    const NAME: &'static str = "UInt64Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.Int32Value`
impl Encode for i32 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int32::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            int32::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for i32 {
    fn new_in(_arena: &Arena) -> Self {
        0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            int32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.Int32Value`
impl Name for i32 {
    const NAME: &'static str = "Int32Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.Int64Value`
impl Encode for i64 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int64::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            int64::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for i64 {
    fn new_in(_arena: &Arena) -> Self {
        0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            int64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.Int64Value`
impl Name for i64 {
    const NAME: &'static str = "Int64Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.FloatValue`
impl Encode for f32 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0.0 {
            float::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0.0 {
            float::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for f32 {
    fn new_in(_arena: &Arena) -> Self {
        0.0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            float::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.FloatValue`
impl Name for f32 {
    const NAME: &'static str = "FloatValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.DoubleValue`
impl Encode for f64 {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0.0 {
            double::encode(1, self, buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0.0 {
            double::encoded_len(1, self)
        } else {
            0
        }
    }
}

impl Decode<'_> for f64 {
    fn new_in(_arena: &Arena) -> Self {
        0.0
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            double::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.DoubleValue`
impl Name for f64 {
    const NAME: &'static str = "DoubleValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.StringValue`
impl Encode for String {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            string::encode(1, self.as_str(), buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            string::encoded_len(1, self.as_str())
        } else {
            0
        }
    }
}

impl Decode<'_> for String {
    fn new_in(_arena: &Arena) -> Self {
        String::new()
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            // Inline string decoding for owned String type
            use crate::encoding::{check_wire_type, decode_varint};
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_varint(buf)? as usize;
            self.clear();
            self.reserve(len);
            unsafe {
                self.as_mut_vec().resize(len, 0);
                buf.copy_to_slice(self.as_mut_vec());
            }
            if core::str::from_utf8(self.as_bytes()).is_err() {
                return Err(DecodeError::new("invalid UTF-8"));
            }
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.StringValue`
impl Name for String {
    const NAME: &'static str = "StringValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl Encode for Vec<u8> {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            bytes::encode(1, self.as_slice(), buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            bytes::encoded_len(1, self.as_slice())
        } else {
            0
        }
    }
}

impl Decode<'_> for Vec<u8> {
    fn new_in(_arena: &Arena) -> Self {
        Vec::new()
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            // Inline bytes decoding for owned Vec<u8> type
            use crate::encoding::{check_wire_type, decode_varint};
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_varint(buf)? as usize;
            self.clear();
            self.resize(len, 0);
            buf.copy_to_slice(self);
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.BytesValue`
impl Name for Vec<u8> {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl Encode for Bytes {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            bytes::encode(1, self.as_ref(), buf)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            bytes::encoded_len(1, self.as_ref())
        } else {
            0
        }
    }
}

impl Decode<'_> for Bytes {
    fn new_in(_arena: &Arena) -> Self {
        Bytes::new()
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            // Inline bytes decoding for Bytes type
            use crate::encoding::{check_wire_type, decode_varint};
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_varint(buf)? as usize;
            *self = buf.copy_to_bytes(len);
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}

/// `google.protobuf.BytesValue`
impl Name for Bytes {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.Empty`
impl Encode for () {
    fn encode_raw(&self, _buf: &mut impl BufMut) {}
    fn encoded_len(&self) -> usize {
        0
    }
}

impl Decode<'_> for () {
    fn new_in(_arena: &Arena) -> Self {
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        _arena: &Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        skip_field(wire_type, tag, buf, ctx)
    }
}

/// `google.protobuf.Empty`
impl Name for () {
    const NAME: &'static str = "Empty";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
fn googleapis_type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impl_name() {
        assert_eq!("BoolValue", bool::NAME);
        assert_eq!("google.protobuf", bool::PACKAGE);
        assert_eq!("google.protobuf.BoolValue", bool::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.BoolValue",
            bool::type_url()
        );

        assert_eq!("UInt32Value", u32::NAME);
        assert_eq!("google.protobuf", u32::PACKAGE);
        assert_eq!("google.protobuf.UInt32Value", u32::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.UInt32Value",
            u32::type_url()
        );

        assert_eq!("UInt64Value", u64::NAME);
        assert_eq!("google.protobuf", u64::PACKAGE);
        assert_eq!("google.protobuf.UInt64Value", u64::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.UInt64Value",
            u64::type_url()
        );

        assert_eq!("Int32Value", i32::NAME);
        assert_eq!("google.protobuf", i32::PACKAGE);
        assert_eq!("google.protobuf.Int32Value", i32::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.Int32Value",
            i32::type_url()
        );

        assert_eq!("Int64Value", i64::NAME);
        assert_eq!("google.protobuf", i64::PACKAGE);
        assert_eq!("google.protobuf.Int64Value", i64::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.Int64Value",
            i64::type_url()
        );

        assert_eq!("FloatValue", f32::NAME);
        assert_eq!("google.protobuf", f32::PACKAGE);
        assert_eq!("google.protobuf.FloatValue", f32::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.FloatValue",
            f32::type_url()
        );

        assert_eq!("DoubleValue", f64::NAME);
        assert_eq!("google.protobuf", f64::PACKAGE);
        assert_eq!("google.protobuf.DoubleValue", f64::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.DoubleValue",
            f64::type_url()
        );

        assert_eq!("StringValue", String::NAME);
        assert_eq!("google.protobuf", String::PACKAGE);
        assert_eq!("google.protobuf.StringValue", String::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.StringValue",
            String::type_url()
        );

        assert_eq!("BytesValue", Vec::<u8>::NAME);
        assert_eq!("google.protobuf", Vec::<u8>::PACKAGE);
        assert_eq!("google.protobuf.BytesValue", Vec::<u8>::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.BytesValue",
            Vec::<u8>::type_url()
        );

        assert_eq!("BytesValue", Bytes::NAME);
        assert_eq!("google.protobuf", Bytes::PACKAGE);
        assert_eq!("google.protobuf.BytesValue", Bytes::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.BytesValue",
            Bytes::type_url()
        );

        assert_eq!("Empty", <()>::NAME);
        assert_eq!("google.protobuf", <()>::PACKAGE);
        assert_eq!("google.protobuf.Empty", <()>::full_name());
        assert_eq!(
            "type.googleapis.com/google.protobuf.Empty",
            <()>::type_url()
        );
    }
}
