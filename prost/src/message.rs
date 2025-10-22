#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use bytes::{Buf, BufMut};

use crate::arena::Arena;
use crate::encoding::varint::{encode_varint, encoded_len_varint};
use crate::encoding::wire_type::WireType;
use crate::encoding::{decode_key, message, DecodeContext};
use crate::DecodeError;
use crate::EncodeError;

/// A Protocol Buffers message with arena-allocated lifetime.
///
/// All messages have a lifetime parameter `'arena` that ties them to the
/// arena from which they were decoded. This enables zero-copy deserialization
/// by allocating all message data from the arena.
///
/// # Lifetimes
///
/// The `'arena` lifetime represents the lifetime of the arena from which the
/// message was allocated. Messages cannot outlive their arena.
///
/// # Examples
///
/// ```ignore
/// use prost::{Message, Arena};
///
/// let arena = Arena::new();
/// let msg = MyMessage::decode(bytes, &arena)?;
/// // msg is tied to arena lifetime
/// // When arena drops, all message data is freed
/// ```
pub trait Message<'arena>: Sized + Send + Sync + 'arena {
    /// Creates a new empty message initialized with the arena.
    ///
    /// This is used internally during decoding to create a message that can
    /// accumulate repeated fields using arena-allocated storage.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn new_in(arena: &'arena Arena) -> Self;

    /// Encodes the message to a buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn encode_raw(&self, buf: &mut impl BufMut);

    /// Decodes a field from a buffer, and merges it into `self`.
    ///
    /// The arena is used to allocate any variable-length data (strings, bytes,
    /// repeated fields, etc.).
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;

    /// Returns the encoded length of the message without a length delimiter.
    fn encoded_len(&self) -> usize;

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let required = self.encoded_len();
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message to a newly allocated buffer.
    fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode_raw(&mut buf);
        buf
    }

    /// Encodes the message with a length-delimiter to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode_length_delimited(&self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let len = self.encoded_len();
        let required = len + encoded_len_varint(len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        encode_varint(len as u64, buf);
        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message with a length-delimiter to a newly allocated buffer.
    fn encode_length_delimited_to_vec(&self) -> Vec<u8> {
        let len = self.encoded_len();
        let mut buf = Vec::with_capacity(len + encoded_len_varint(len as u64));

        encode_varint(len as u64, &mut buf);
        self.encode_raw(&mut buf);
        buf
    }

    /// Decodes an instance of the message from a buffer using the provided arena.
    ///
    /// All variable-length data (strings, bytes, repeated fields, maps, nested
    /// messages) will be allocated from the arena. The returned message has a
    /// lifetime tied to the arena.
    ///
    /// The entire buffer will be consumed.
    fn decode(mut buf: impl Buf, arena: &'arena Arena) -> Result<Self, DecodeError> {
        let mut message = Self::new_in(arena);
        Self::merge(&mut message, &mut buf, arena).map(|_| message)
    }

    /// Decodes a length-delimited instance of the message from the buffer.
    fn decode_length_delimited(buf: impl Buf, arena: &'arena Arena) -> Result<Self, DecodeError> {
        let mut message = Self::new_in(arena);
        message.merge_length_delimited(buf, arena)?;
        Ok(message)
    }

    /// Decodes an instance of the message from a buffer, and merges it into `self`.
    ///
    /// The arena is used to allocate any variable-length data.
    ///
    /// The entire buffer will be consumed.
    fn merge(&mut self, mut buf: impl Buf, arena: &'arena Arena) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            self.merge_field(tag, wire_type, &mut buf, arena, ctx.clone())?;
        }
        Ok(())
    }

    /// Decodes a length-delimited instance of the message from buffer, and
    /// merges it into `self`.
    fn merge_length_delimited(&mut self, mut buf: impl Buf, arena: &'arena Arena) -> Result<(), DecodeError> {
        message::merge(
            WireType::LengthDelimited,
            self,
            &mut buf,
            arena,
            DecodeContext::default(),
        )
    }

    /// Clears the message, resetting all fields to their default.
    fn clear(&mut self);
}

// Note: Box<M> impl removed - boxes don't work well with arena lifetimes
// Users should allocate messages directly in the arena instead

// Note: We don't implement Message for &T or Option<&T> because:
// 1. It causes infinite recursion (dereferencing and reborrowing)
// 2. View types implement Message directly now
// 3. Oneofs with recursive types need custom handling in the derive macro

#[cfg(test)]
mod tests {
    use super::*;

    // Message trait is now lifetime-parameterized, making it non-object-safe
    // This is acceptable as we're not using trait objects for messages
}
