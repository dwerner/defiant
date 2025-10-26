//! Utility functions and types for encoding and decoding Protobuf types.
//!
//! This module contains the encoding and decoding primatives for Protobuf as described in
//! <https://protobuf.dev/programming-guides/encoding/>.
//!
//! This module is `pub`, but is only for prost internal use. The `prost-derive` crate needs access for its `Message` implementations.

use alloc::format;
use alloc::vec::Vec;
use core::str;

use ::bytes::{Buf, BufMut, Bytes};

use crate::DecodeError;

pub mod varint;
pub use varint::{decode_varint, encode_varint, encoded_len_varint};

pub mod length_delimiter;
pub use length_delimiter::{
    decode_length_delimiter, encode_length_delimiter, length_delimiter_len,
};

pub mod wire_type;
pub use wire_type::{check_wire_type, WireType};

/// Additional information passed to every decode/merge function.
///
/// The context should be passed by value and can be freely cloned. When passing
/// to a function which is decoding a nested object, then use `enter_recursion`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "no-recursion-limit", derive(Default))]
pub struct DecodeContext {
    /// How many times we can recurse in the current decode stack before we hit
    /// the recursion limit.
    ///
    /// The recursion limit is defined by `RECURSION_LIMIT` and cannot be
    /// customized. The recursion limit can be ignored by building the Prost
    /// crate with the `no-recursion-limit` feature.
    #[cfg(not(feature = "no-recursion-limit"))]
    recurse_count: u32,
}

#[cfg(not(feature = "no-recursion-limit"))]
impl Default for DecodeContext {
    #[inline]
    fn default() -> DecodeContext {
        DecodeContext {
            recurse_count: crate::RECURSION_LIMIT,
        }
    }
}

impl DecodeContext {
    /// Call this function before recursively decoding.
    ///
    /// There is no `exit` function since this function creates a new `DecodeContext`
    /// to be used at the next level of recursion. Continue to use the old context
    // at the previous level of recursion.
    #[cfg(not(feature = "no-recursion-limit"))]
    #[inline]
    pub fn enter_recursion(&self) -> DecodeContext {
        DecodeContext {
            recurse_count: self.recurse_count - 1,
        }
    }

    #[cfg(feature = "no-recursion-limit")]
    #[inline]
    pub fn enter_recursion(&self) -> DecodeContext {
        DecodeContext {}
    }

    /// Checks whether the recursion limit has been reached in the stack of
    /// decodes described by the `DecodeContext` at `self.ctx`.
    ///
    /// Returns `Ok<()>` if it is ok to continue recursing.
    /// Returns `Err<DecodeError>` if the recursion limit has been reached.
    #[cfg(not(feature = "no-recursion-limit"))]
    #[inline]
    pub fn limit_reached(&self) -> Result<(), DecodeError> {
        if self.recurse_count == 0 {
            Err(DecodeError::new("recursion limit reached"))
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "no-recursion-limit")]
    #[inline]
    pub fn limit_reached(&self) -> Result<(), DecodeError> {
        Ok(())
    }
}

pub const MIN_TAG: u32 = 1;
pub const MAX_TAG: u32 = (1 << 29) - 1;

/// Encodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline]
pub fn encode_key(tag: u32, wire_type: WireType, buf: &mut impl BufMut) {
    debug_assert!((MIN_TAG..=MAX_TAG).contains(&tag));
    let key = (tag << 3) | wire_type as u32;
    encode_varint(u64::from(key), buf);
}

/// Decodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline(always)]
pub fn decode_key(buf: &mut impl Buf) -> Result<(u32, WireType), DecodeError> {
    let key = decode_varint(buf)?;
    if key > u64::from(u32::MAX) {
        return Err(DecodeError::new(format!("invalid key value: {key}")));
    }
    let wire_type = WireType::try_from(key & 0x07)?;
    let tag = key as u32 >> 3;

    if tag < MIN_TAG {
        return Err(DecodeError::new("invalid tag value: 0"));
    }

    Ok((tag, wire_type))
}

/// Returns the width of an encoded Protobuf field key with the given tag.
/// The returned width will be between 1 and 5 bytes (inclusive).
#[inline]
pub const fn key_len(tag: u32) -> usize {
    encoded_len_varint((tag << 3) as u64)
}

/// Helper function which abstracts reading a length delimiter prefix followed
/// by decoding values until the length of bytes is exhausted.
pub fn merge_loop<T, M, B>(
    value: &mut T,
    buf: &mut B,
    ctx: DecodeContext,
    mut merge: M,
) -> Result<(), DecodeError>
where
    M: FnMut(&mut T, &mut B, DecodeContext) -> Result<(), DecodeError>,
    B: Buf,
{
    let len = decode_varint(buf)?;
    let remaining = buf.remaining();
    if len > remaining as u64 {
        return Err(DecodeError::new("buffer underflow"));
    }

    let limit = remaining - len as usize;
    while buf.remaining() > limit {
        merge(value, buf, ctx.clone())?;
    }

    if buf.remaining() != limit {
        return Err(DecodeError::new("delimited length exceeded"));
    }
    Ok(())
}

pub fn skip_field(
    wire_type: WireType,
    tag: u32,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    ctx.limit_reached()?;
    let len = match wire_type {
        WireType::Varint => decode_varint(buf).map(|_| 0)?,
        WireType::ThirtyTwoBit => 4,
        WireType::SixtyFourBit => 8,
        WireType::LengthDelimited => decode_varint(buf)?,
        WireType::StartGroup => loop {
            let (inner_tag, inner_wire_type) = decode_key(buf)?;
            match inner_wire_type {
                WireType::EndGroup => {
                    if inner_tag != tag {
                        return Err(DecodeError::new("unexpected end group tag"));
                    }
                    break 0;
                }
                _ => skip_field(inner_wire_type, inner_tag, buf, ctx.enter_recursion())?,
            }
        },
        WireType::EndGroup => return Err(DecodeError::new("unexpected end group tag")),
    };

    if len > buf.remaining() as u64 {
        return Err(DecodeError::new("buffer underflow"));
    }

    buf.advance(len as usize);
    Ok(())
}

/// Helper macro which emits an `encode_repeated` function for the type.
macro_rules! encode_repeated {
    ($ty:ty) => {
        pub fn encode_repeated(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
            for value in values {
                encode(tag, value, buf);
            }
        }
    };
}

/// Helper macro which emits a `merge_repeated` function for the numeric type.
macro_rules! merge_repeated_numeric {
    ($ty:ty,
     $wire_type:expr,
     $merge:ident,
     $merge_repeated:ident) => {
        pub fn $merge_repeated<V>(
            wire_type: WireType,
            values: &mut V,
            buf: &mut impl Buf,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError>
        where
            V: core::ops::DerefMut<Target = [$ty]> + core::iter::Extend<$ty>,
        {
            if wire_type == WireType::LengthDelimited {
                // Packed.
                merge_loop(values, buf, ctx, |values, buf, ctx| {
                    let mut value = Default::default();
                    $merge($wire_type, &mut value, buf, ctx)?;
                    values.extend(core::iter::once(value));
                    Ok(())
                })
            } else {
                // Unpacked.
                check_wire_type($wire_type, wire_type)?;
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf, ctx)?;
                values.extend(core::iter::once(value));
                Ok(())
            }
        }
    };
}

/// Macro which emits a module containing a set of encoding functions for a
/// variable width numeric type.
macro_rules! varint {
    ($ty:ty,
     $proto_ty:ident) => (
        varint!($ty,
                $proto_ty,
                to_uint64(value) { *value as u64 },
                from_uint64(value) { value as $ty });
    );

    ($ty:ty,
     $proto_ty:ident,
     to_uint64($to_uint64_value:ident) $to_uint64:expr,
     from_uint64($from_uint64_value:ident) $from_uint64:expr) => (

         pub mod $proto_ty {
            use crate::encoding::*;

            pub fn encode(tag: u32, $to_uint64_value: &$ty, buf: &mut impl BufMut) {
                encode_key(tag, WireType::Varint, buf);
                encode_varint($to_uint64, buf);
            }

            pub fn merge(wire_type: WireType, value: &mut $ty, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
                check_wire_type(WireType::Varint, wire_type)?;
                let $from_uint64_value = decode_varint(buf)?;
                *value = $from_uint64;
                Ok(())
            }

            encode_repeated!($ty);

            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() { return; }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len: usize = values.iter().map(|$to_uint64_value| {
                    encoded_len_varint($to_uint64)
                }).sum();
                encode_varint(len as u64, buf);

                for $to_uint64_value in values {
                    encode_varint($to_uint64, buf);
                }
            }

            merge_repeated_numeric!($ty, WireType::Varint, merge, merge_repeated);

            #[inline]
            pub fn encoded_len(tag: u32, $to_uint64_value: &$ty) -> usize {
                key_len(tag) + encoded_len_varint($to_uint64)
            }

            #[inline]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                key_len(tag) * values.len() + values.iter().map(|$to_uint64_value| {
                    encoded_len_varint($to_uint64)
                }).sum::<usize>()
            }

            #[inline]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = values.iter()
                                    .map(|$to_uint64_value| encoded_len_varint($to_uint64))
                                    .sum::<usize>();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            #[cfg(test)]
            mod test {
                use proptest::prelude::*;

                use crate::encoding::$proto_ty::*;
                use crate::encoding::test::{
                    check_collection_type,
                    check_type,
                };

                proptest! {
                    #[test]
                    fn check(value: $ty, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::Varint,
                                   encode, merge, encoded_len)?;
                    }
                    #[test]
                    fn check_repeated(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_collection_type(value, tag, WireType::Varint,
                                              encode_repeated, merge_repeated,
                                              encoded_len_repeated)?;
                    }
                    #[test]
                    fn check_packed(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::LengthDelimited,
                                   encode_packed, merge_repeated,
                                   encoded_len_packed)?;
                    }
                }
            }
         }

    );
}
varint!(bool, bool,
        to_uint64(value) u64::from(*value),
        from_uint64(value) value != 0);
varint!(i32, int32);
varint!(i64, int64);
varint!(u32, uint32);
varint!(u64, uint64);
varint!(i32, sint32,
to_uint64(value) {
    ((value << 1) ^ (value >> 31)) as u32 as u64
},
from_uint64(value) {
    let value = value as u32;
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
});
varint!(i64, sint64,
to_uint64(value) {
    ((value << 1) ^ (value >> 63)) as u64
},
from_uint64(value) {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
});

/// Macro which emits a module containing a set of encoding functions for a
/// fixed width numeric type.
macro_rules! fixed_width {
    ($ty:ty,
     $width:expr,
     $wire_type:expr,
     $proto_ty:ident,
     $put:ident,
     $get:ident) => {
        pub mod $proto_ty {
            use crate::encoding::*;

            pub fn encode(tag: u32, value: &$ty, buf: &mut impl BufMut) {
                encode_key(tag, $wire_type, buf);
                buf.$put(*value);
            }

            pub fn merge(
                wire_type: WireType,
                value: &mut $ty,
                buf: &mut impl Buf,
                _ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                check_wire_type($wire_type, wire_type)?;
                if buf.remaining() < $width {
                    return Err(DecodeError::new("buffer underflow"));
                }
                *value = buf.$get();
                Ok(())
            }

            encode_repeated!($ty);

            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() {
                    return;
                }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len = values.len() as u64 * $width;
                encode_varint(len as u64, buf);

                for value in values {
                    buf.$put(*value);
                }
            }

            merge_repeated_numeric!($ty, $wire_type, merge, merge_repeated);

            #[inline]
            pub fn encoded_len(tag: u32, _: &$ty) -> usize {
                key_len(tag) + $width
            }

            #[inline]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                (key_len(tag) + $width) * values.len()
            }

            #[inline]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = $width * values.len();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            #[cfg(test)]
            mod test {
                use proptest::prelude::*;

                use super::super::test::{check_collection_type, check_type};
                use super::*;

                proptest! {
                    #[test]
                    fn check(value: $ty, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, $wire_type,
                                   encode, merge, encoded_len)?;
                    }
                    #[test]
                    fn check_repeated(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_collection_type(value, tag, $wire_type,
                                              encode_repeated, merge_repeated,
                                              encoded_len_repeated)?;
                    }
                    #[test]
                    fn check_packed(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::LengthDelimited,
                                   encode_packed, merge_repeated,
                                   encoded_len_packed)?;
                    }
                }
            }
        }
    };
}
fixed_width!(
    f32,
    4,
    WireType::ThirtyTwoBit,
    float,
    put_f32_le,
    get_f32_le
);
fixed_width!(
    f64,
    8,
    WireType::SixtyFourBit,
    double,
    put_f64_le,
    get_f64_le
);
fixed_width!(
    u32,
    4,
    WireType::ThirtyTwoBit,
    fixed32,
    put_u32_le,
    get_u32_le
);
fixed_width!(
    u64,
    8,
    WireType::SixtyFourBit,
    fixed64,
    put_u64_le,
    get_u64_le
);
fixed_width!(
    i32,
    4,
    WireType::ThirtyTwoBit,
    sfixed32,
    put_i32_le,
    get_i32_le
);
fixed_width!(
    i64,
    8,
    WireType::SixtyFourBit,
    sfixed64,
    put_i64_le,
    get_i64_le
);

/// Macro which emits encoding functions for a length-delimited type.
#[allow(unused_macros)]
macro_rules! length_delimited {
    ($ty:ty) => {
        encode_repeated!($ty);

        pub fn merge_repeated(
            wire_type: WireType,
            values: &mut Vec<$ty>,
            buf: &mut impl Buf,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError> {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let mut value = Default::default();
            merge(wire_type, &mut value, buf, ctx)?;
            values.push(value);
            Ok(())
        }

        #[inline]
        #[allow(clippy::ptr_arg)]
        pub fn encoded_len(tag: u32, value: &$ty) -> usize {
            key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
        }

        #[inline]
        pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
            key_len(tag) * values.len()
                + values
                    .iter()
                    .map(|value| encoded_len_varint(value.len() as u64) + value.len())
                    .sum::<usize>()
        }
    };
}

pub mod string {
    use super::*;
    use crate::Arena;

    /// Encode a string slice
    pub fn encode(tag: u32, value: &str, buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        buf.put_slice(value.as_bytes());
    }

    /// Decodes a string and allocates it in the provided arena.
    ///
    /// Returns a reference to the decoded string with the arena's lifetime.
    pub fn merge_arena<'arena>(
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        _ctx: DecodeContext,
    ) -> Result<&'arena str, DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;

        // Decode the length
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        let len = len as usize;

        // Allocate uninitialized buffer and copy directly (single copy, no zero-fill)
        let mut vec = arena.new_vec_with_capacity::<u8>(len);
        unsafe {
            vec.copy_from_buf_uninit(buf, len);
        }
        let bytes = vec.freeze();

        // Validate UTF-8 and convert to &str
        str::from_utf8(bytes)
            .map_err(|_| DecodeError::new("invalid string value: data is not UTF-8 encoded"))
    }

    /// Encode repeated string slices
    pub fn encode_repeated(tag: u32, values: &[&str], buf: &mut impl BufMut) {
        for value in values {
            encode(tag, value, buf);
        }
    }

    /// Merge repeated string into arena ArenaVec
    pub fn merge_repeated_arena<'a>(
        wire_type: WireType,
        values: &mut crate::arena::ArenaVec<'a, &'a str>,
        buf: &mut impl Buf,
        arena: &'a crate::Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let value = merge_arena(wire_type, buf, arena, ctx)?;
        values.push(value);
        Ok(())
    }

    #[inline]
    pub fn encoded_len(tag: u32, value: &str) -> usize {
        key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
    }

    #[inline]
    pub fn encoded_len_repeated(tag: u32, values: &[&str]) -> usize {
        key_len(tag) * values.len()
            + values
                .iter()
                .map(|value| encoded_len_varint(value.len() as u64) + value.len())
                .sum::<usize>()
    }

    // Tests removed - string encoding only supports arena-allocated &str, not owned String
}

pub trait BytesAdapter: sealed::BytesAdapter {}

mod sealed {
    use super::{Buf, BufMut};

    pub trait BytesAdapter: Default + Sized + 'static {
        fn len(&self) -> usize;

        /// Replace contents of this buffer with the contents of another buffer.
        fn replace_with(&mut self, buf: impl Buf);

        /// Appends this buffer to the (contents of) other buffer.
        fn append_to(&self, buf: &mut impl BufMut);

        fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }
}

impl BytesAdapter for Bytes {}

impl sealed::BytesAdapter for Bytes {
    fn len(&self) -> usize {
        Buf::remaining(self)
    }

    fn replace_with(&mut self, mut buf: impl Buf) {
        *self = buf.copy_to_bytes(buf.remaining());
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.clone())
    }
}

impl BytesAdapter for Vec<u8> {}

impl sealed::BytesAdapter for Vec<u8> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn replace_with(&mut self, buf: impl Buf) {
        self.clear();
        self.reserve(buf.remaining());
        self.put(buf);
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.as_slice())
    }
}

pub mod bytes {
    use super::*;
    use crate::Arena;

    /// Encode a byte slice
    pub fn encode(tag: u32, value: &[u8], buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        buf.put_slice(value);
    }

    /// Decodes bytes and allocates them in the provided arena.
    ///
    /// Returns a reference to the decoded bytes with the arena's lifetime.
    pub fn merge_arena<'arena>(
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        _ctx: DecodeContext,
    ) -> Result<&'arena [u8], DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;

        // Decode the length
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        let len = len as usize;

        // Allocate uninitialized buffer and copy directly (single copy, no zero-fill)
        let mut vec = arena.new_vec_with_capacity::<u8>(len);
        unsafe {
            vec.copy_from_buf_uninit(buf, len);
        }
        Ok(vec.freeze())
    }

    /// Encode repeated byte slices
    pub fn encode_repeated(tag: u32, values: &[&[u8]], buf: &mut impl BufMut) {
        for value in values {
            encode(tag, value, buf);
        }
    }

    #[inline]
    pub fn encoded_len(tag: u32, value: &[u8]) -> usize {
        key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
    }

    #[inline]
    pub fn encoded_len_repeated(tag: u32, values: &[&[u8]]) -> usize {
        key_len(tag) * values.len()
            + values
                .iter()
                .map(|value| encoded_len_varint(value.len() as u64) + value.len())
                .sum::<usize>()
    }

    /// Merge repeated bytes into arena ArenaVec
    pub fn merge_repeated_arena<'a>(
        wire_type: WireType,
        values: &mut crate::arena::ArenaVec<'a, &'a [u8]>,
        buf: &mut impl Buf,
        arena: &'a crate::Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let value = merge_arena(wire_type, buf, arena, ctx)?;
        values.push(value);
        Ok(())
    }

    // Tests removed - bytes encoding only supports arena-allocated &[u8], not owned Vec/Bytes
}

pub mod message {
    use super::*;
    use crate::Arena;
    use crate::{Decode, Encode};

    pub fn encode<M>(tag: u32, msg: &M, buf: &mut impl BufMut)
    where
        M: Encode,
    {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(msg.encoded_len() as u64, buf);
        msg.encode_raw(buf);
    }

    pub fn merge<'arena, M, B>(
        wire_type: WireType,
        msg: &mut M,
        buf: &mut B,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Decode<'arena>,
        B: Buf,
    {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        ctx.limit_reached()?;
        merge_loop(
            msg,
            buf,
            ctx.enter_recursion(),
            |msg: &mut M, buf: &mut B, ctx| {
                let (tag, wire_type) = decode_key(buf)?;
                msg.merge_field(tag, wire_type, buf, arena, ctx)
            },
        )
    }

    pub fn encode_repeated<M>(tag: u32, messages: &[M], buf: &mut impl BufMut)
    where
        M: Encode,
    {
        for msg in messages {
            encode(tag, msg, buf);
        }
    }

    pub fn merge_repeated<'arena, M>(
        wire_type: WireType,
        messages: &mut crate::arena::ArenaVec<'arena, M>,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Decode<'arena>,
    {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut msg = M::new_in(arena);
        merge(WireType::LengthDelimited, &mut msg, buf, arena, ctx)?;
        messages.push(msg);
        Ok(())
    }

    #[inline]
    pub fn encoded_len<M>(tag: u32, msg: &M) -> usize
    where
        M: Encode,
    {
        let len = msg.encoded_len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }

    #[inline]
    pub fn encoded_len_repeated<M>(tag: u32, messages: &[M]) -> usize
    where
        M: Encode,
    {
        key_len(tag) * messages.len()
            + messages
                .iter()
                .map(|msg: &M| msg.encoded_len())
                .map(|len| len + encoded_len_varint(len as u64))
                .sum::<usize>()
    }
}

pub mod group {
    use super::*;
    use crate::Arena;
    use crate::{Decode, Encode};

    pub fn encode<M>(tag: u32, msg: &M, buf: &mut impl BufMut)
    where
        M: Encode,
    {
        encode_key(tag, WireType::StartGroup, buf);
        msg.encode_raw(buf);
        encode_key(tag, WireType::EndGroup, buf);
    }

    pub fn merge<'arena, M>(
        tag: u32,
        wire_type: WireType,
        msg: &mut M,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Decode<'arena>,
    {
        check_wire_type(WireType::StartGroup, wire_type)?;

        ctx.limit_reached()?;
        loop {
            let (field_tag, field_wire_type) = decode_key(buf)?;
            if field_wire_type == WireType::EndGroup {
                if field_tag != tag {
                    return Err(DecodeError::new("unexpected end group tag"));
                }
                return Ok(());
            }

            msg.merge_field(
                field_tag,
                field_wire_type,
                buf,
                arena,
                ctx.enter_recursion(),
            )?;
        }
    }

    pub fn encode_repeated<M>(tag: u32, messages: &[M], buf: &mut impl BufMut)
    where
        M: Encode,
    {
        for msg in messages {
            encode(tag, msg, buf);
        }
    }

    pub fn merge_repeated<'arena, M>(
        tag: u32,
        wire_type: WireType,
        messages: &mut crate::arena::ArenaVec<'arena, M>,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Decode<'arena>,
    {
        check_wire_type(WireType::StartGroup, wire_type)?;
        let mut msg = M::new_in(arena);
        merge(tag, WireType::StartGroup, &mut msg, buf, arena, ctx)?;
        messages.push(msg);
        Ok(())
    }

    #[inline]
    pub fn encoded_len<M>(tag: u32, msg: &M) -> usize
    where
        M: Encode,
    {
        2 * key_len(tag) + msg.encoded_len()
    }

    #[inline]
    pub fn encoded_len_repeated<M>(tag: u32, messages: &[M]) -> usize
    where
        M: Encode,
    {
        2 * key_len(tag) * messages.len() + messages.iter().map(Encode::encoded_len).sum::<usize>()
    }
}

/// Arena-allocated map encoding functions.
///
/// These functions work with ArenaVec during decoding (accumulating entries)
/// and with slices during encoding (from ArenaMap).
pub mod arena_map {
    use crate::arena::ArenaVec;
    use crate::encoding::*;

    /// Generic protobuf map merge function for arena-allocated maps.
    ///
    /// Accumulates entries into a ArenaVec during decoding.
    /// Caller must provide initial key and value instances.
    pub fn merge_with_defaults<'arena, K, V, B, KM, VM>(
        key_merge: KM,
        val_merge: VM,
        key_default: K,
        val_default: V,
        values: &mut ArenaVec<'arena, (K, V)>,
        buf: &mut B,
        arena: &'arena crate::Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        KM: Fn(
            WireType,
            &mut K,
            &mut B,
            &'arena crate::Arena,
            DecodeContext,
        ) -> Result<(), DecodeError>,
        VM: Fn(
            WireType,
            &mut V,
            &mut B,
            &'arena crate::Arena,
            DecodeContext,
        ) -> Result<(), DecodeError>,
    {
        let mut key = key_default;
        let mut val = val_default;
        ctx.limit_reached()?;
        merge_loop(
            &mut (&mut key, &mut val),
            buf,
            ctx.enter_recursion(),
            |&mut (ref mut key, ref mut val), buf, ctx| {
                let (tag, wire_type) = decode_key(buf)?;
                match tag {
                    1 => key_merge(wire_type, key, buf, arena, ctx),
                    2 => val_merge(wire_type, val, buf, arena, ctx),
                    _ => skip_field(wire_type, tag, buf, ctx),
                }
            },
        )?;
        values.push((key, val));

        Ok(())
    }

    /// Map merge function for message values - DEPRECATED
    ///
    /// This function is no longer used. Map fields with message values now use
    /// custom inline merge code generated in the proc macro to handle Builder/View types properly.
    pub fn merge_message<'arena, K, VBuilder, VView, B, KM, VM, VN, VF>(
        _key_merge: KM,
        _val_merge: VM,
        _val_new: VN,
        _val_freeze: VF,
        _key_default: K,
        _values: &mut ArenaVec<'arena, (K, VView)>,
        _buf: &mut B,
        _arena: &'arena crate::Arena,
        _ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        KM: Fn(
            WireType,
            &mut K,
            &mut B,
            &'arena crate::Arena,
            DecodeContext,
        ) -> Result<(), DecodeError>,
        VM: Fn(
            WireType,
            &mut VBuilder,
            &mut B,
            &'arena crate::Arena,
            DecodeContext,
        ) -> Result<(), DecodeError>,
        VN: Fn(&'arena crate::Arena) -> VBuilder,
        VF: Fn(VBuilder) -> VView,
    {
        panic!("merge_message is deprecated - use custom inline merge code instead")
    }

    /// Generic protobuf map encode function with overridden key and value defaults.
    pub fn encode_with_defaults<K, V, B, KE, KL, VE, VL>(
        key_encode: KE,
        key_encoded_len: KL,
        val_encode: VE,
        val_encoded_len: VL,
        key_default: &K,
        val_default: &V,
        tag: u32,
        values: &[(K, V)],
        buf: &mut B,
    ) where
        K: PartialEq,
        V: PartialEq,
        B: BufMut,
        KE: Fn(u32, &K, &mut B),
        KL: Fn(u32, &K) -> usize,
        VE: Fn(u32, &V, &mut B),
        VL: Fn(u32, &V) -> usize,
    {
        for (key, val) in values.iter() {
            let skip_key = key == key_default;
            let skip_val = val == val_default;

            let len = (if skip_key { 0 } else { key_encoded_len(1, key) })
                + (if skip_val { 0 } else { val_encoded_len(2, val) });

            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(len as u64, buf);
            if !skip_key {
                key_encode(1, key, buf);
            }
            if !skip_val {
                val_encode(2, val, buf);
            }
        }
    }

    /// Generic protobuf map encode function for message values that don't implement Default.
    ///
    /// Always encodes all values (no default-value optimization for messages).
    pub fn encode_message<K, V, B, KE, KL, VE, VL>(
        key_encode: KE,
        key_encoded_len: KL,
        val_encode: VE,
        val_encoded_len: VL,
        key_default: &K,
        tag: u32,
        values: &[(K, V)],
        buf: &mut B,
    ) where
        K: PartialEq,
        B: BufMut,
        KE: Fn(u32, &K, &mut B),
        KL: Fn(u32, &K) -> usize,
        VE: Fn(u32, &V, &mut B),
        VL: Fn(u32, &V) -> usize,
    {
        for (key, val) in values.iter() {
            let skip_key = key == key_default;

            let len =
                (if skip_key { 0 } else { key_encoded_len(1, key) }) + val_encoded_len(2, val);

            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(len as u64, buf);
            if !skip_key {
                key_encode(1, key, buf);
            }
            // Always encode the value (no default comparison for messages)
            val_encode(2, val, buf);
        }
    }

    /// Generic protobuf map encoded length function with key and value defaults.
    pub fn encoded_len_with_defaults<K, V, KL, VL>(
        key_encoded_len: KL,
        val_encoded_len: VL,
        key_default: &K,
        val_default: &V,
        tag: u32,
        values: &[(K, V)],
    ) -> usize
    where
        K: PartialEq,
        V: PartialEq,
        KL: Fn(u32, &K) -> usize,
        VL: Fn(u32, &V) -> usize,
    {
        key_len(tag) * values.len()
            + values
                .iter()
                .map(|(key, val)| {
                    let len = (if key == key_default {
                        0
                    } else {
                        key_encoded_len(1, key)
                    }) + (if val == val_default {
                        0
                    } else {
                        val_encoded_len(2, val)
                    });
                    encoded_len_varint(len as u64) + len
                })
                .sum::<usize>()
    }

    /// Map encoded length function for message values that don't implement Default.
    ///
    /// Always encodes all values (no default-value optimization for messages).
    pub fn encoded_len_message<K, V, KL, VL>(
        key_encoded_len: KL,
        val_encoded_len: VL,
        key_default: &K,
        tag: u32,
        values: &[(K, V)],
    ) -> usize
    where
        K: PartialEq,
        KL: Fn(u32, &K) -> usize,
        VL: Fn(u32, &V) -> usize,
    {
        key_len(tag) * values.len()
            + values
                .iter()
                .map(|(key, val)| {
                    // Always encode the value (no default comparison for messages)
                    // Only skip the key if it equals the key default
                    let len = (if key == key_default {
                        0
                    } else {
                        key_encoded_len(1, key)
                    }) + val_encoded_len(2, val);
                    encoded_len_varint(len as u64) + len
                })
                .sum::<usize>()
    }
}

#[cfg(test)]
mod test {
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;
    use core::borrow::Borrow;
    use core::fmt::Debug;

    use ::bytes::BytesMut;
    use proptest::{prelude::*, test_runner::TestCaseResult};

    use super::*;

    pub fn check_type<T, B>(
        value: T,
        tag: u32,
        wire_type: WireType,
        encode: fn(u32, &B, &mut BytesMut),
        merge: fn(WireType, &mut T, &mut Bytes, DecodeContext) -> Result<(), DecodeError>,
        encoded_len: fn(u32, &B) -> usize,
    ) -> TestCaseResult
    where
        T: Debug + Default + PartialEq + Borrow<B>,
        B: ?Sized,
    {
        prop_assume!((MIN_TAG..=MAX_TAG).contains(&tag));

        let expected_len = encoded_len(tag, value.borrow());

        let mut buf = BytesMut::with_capacity(expected_len);
        encode(tag, value.borrow(), &mut buf);

        let mut buf = buf.freeze();

        prop_assert_eq!(
            buf.remaining(),
            expected_len,
            "encoded_len wrong; expected: {}, actual: {}",
            expected_len,
            buf.remaining()
        );

        if !buf.has_remaining() {
            // Short circuit for empty packed values.
            return Ok(());
        }

        let (decoded_tag, decoded_wire_type) =
            decode_key(&mut buf).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(
            tag,
            decoded_tag,
            "decoded tag does not match; expected: {}, actual: {}",
            tag,
            decoded_tag
        );

        prop_assert_eq!(
            wire_type,
            decoded_wire_type,
            "decoded wire type does not match; expected: {:?}, actual: {:?}",
            wire_type,
            decoded_wire_type,
        );

        match wire_type {
            WireType::SixtyFourBit if buf.remaining() != 8 => Err(TestCaseError::fail(format!(
                "64bit wire type illegal remaining: {}, tag: {}",
                buf.remaining(),
                tag
            ))),
            WireType::ThirtyTwoBit if buf.remaining() != 4 => Err(TestCaseError::fail(format!(
                "32bit wire type illegal remaining: {}, tag: {}",
                buf.remaining(),
                tag
            ))),
            _ => Ok(()),
        }?;

        let mut roundtrip_value = T::default();
        merge(
            wire_type,
            &mut roundtrip_value,
            &mut buf,
            DecodeContext::default(),
        )
        .map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert!(
            !buf.has_remaining(),
            "expected buffer to be empty, remaining: {}",
            buf.remaining()
        );

        prop_assert_eq!(value, roundtrip_value);

        Ok(())
    }

    pub fn check_collection_type<T, B, E, M, L>(
        value: T,
        tag: u32,
        wire_type: WireType,
        encode: E,
        mut merge: M,
        encoded_len: L,
    ) -> TestCaseResult
    where
        T: Debug + Default + PartialEq + Borrow<B>,
        B: ?Sized,
        E: FnOnce(u32, &B, &mut BytesMut),
        M: FnMut(WireType, &mut T, &mut Bytes, DecodeContext) -> Result<(), DecodeError>,
        L: FnOnce(u32, &B) -> usize,
    {
        prop_assume!((MIN_TAG..=MAX_TAG).contains(&tag));

        let expected_len = encoded_len(tag, value.borrow());

        let mut buf = BytesMut::with_capacity(expected_len);
        encode(tag, value.borrow(), &mut buf);

        let mut buf = buf.freeze();

        prop_assert_eq!(
            buf.remaining(),
            expected_len,
            "encoded_len wrong; expected: {}, actual: {}",
            expected_len,
            buf.remaining()
        );

        let mut roundtrip_value = Default::default();
        while buf.has_remaining() {
            let (decoded_tag, decoded_wire_type) =
                decode_key(&mut buf).map_err(|error| TestCaseError::fail(error.to_string()))?;

            prop_assert_eq!(
                tag,
                decoded_tag,
                "decoded tag does not match; expected: {}, actual: {}",
                tag,
                decoded_tag
            );

            prop_assert_eq!(
                wire_type,
                decoded_wire_type,
                "decoded wire type does not match; expected: {:?}, actual: {:?}",
                wire_type,
                decoded_wire_type
            );

            merge(
                wire_type,
                &mut roundtrip_value,
                &mut buf,
                DecodeContext::default(),
            )
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        }

        prop_assert_eq!(value, roundtrip_value);

        Ok(())
    }

    // Legacy string_merge_invalid_utf8 test removed - tested owned String which we no longer support

    // Legacy map_tests! removed - tested owned HashMap/BTreeMap which we no longer support

    #[test]
    /// `decode_varint` accepts a `Buf`, which can be multiple concatinated buffers.
    /// This test ensures that future optimizations don't break the
    /// `decode_varint` for non-continuous memory.
    fn split_varint_decoding() {
        let mut test_values = Vec::<u64>::with_capacity(10 * 2);
        test_values.push(128);
        for i in 2..9 {
            test_values.push((1 << (7 * i)) - 1);
            test_values.push(1 << (7 * i));
        }

        for v in test_values {
            let mut buf = BytesMut::with_capacity(10);
            encode_varint(v, &mut buf);
            let half_len = buf.len() / 2;
            let len = buf.len();
            // this weird sequence here splits the buffer into two instances of Bytes
            // which we then stitch together with `bytes::buf::Buf::chain`
            // which ensures the varint bytes are not in a single chunk
            let b2 = buf.split_off(half_len);
            let mut c = buf.chain(b2);

            // make sure all the bytes are inside
            assert_eq!(c.remaining(), len);
            // make sure the first chunk is split as we expected
            assert_eq!(c.chunk().len(), half_len);
            assert_eq!(v, decode_varint(&mut c).unwrap());
        }
    }
}
