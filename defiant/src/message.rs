#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use bytes::{Buf, BufMut};

use crate::arena::Arena;
use crate::encoding::varint::{encode_varint, encoded_len_varint};
use crate::encoding::wire_type::WireType;
use crate::encoding::{decode_key, message, DecodeContext};
use crate::DecodeError;
use crate::EncodeError;

/// Trait for encoding protobuf messages.
///
/// This trait is implemented by view types - frozen, immutable message snapshots
/// that can be efficiently encoded to the protobuf wire format.
///
/// Views are created by calling `freeze()` on a builder, or by decoding bytes
/// and immediately freezing.
///
/// # Examples
///
/// ```ignore
/// use defiant::{Encode, Arena};
///
/// let view: MyMessage = ...; // frozen view
/// let bytes = view.encode_to_vec();
/// ```
pub trait Encode {
    /// Encodes the message to a buffer without a length delimiter.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Encode` implementations.
    #[doc(hidden)]
    fn encode_raw(&self, buf: &mut impl BufMut);

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

    /// Encodes the message directly into arena-allocated memory.
    ///
    /// Encodes directly to an ArenaVec in the arena (zero heap allocation),
    /// then freezes it to an immutable slice.
    fn arena_encode<'arena>(&self, arena: &'arena Arena) -> &'arena [u8] {
        let len = self.encoded_len();
        let mut buf = arena.new_vec_with_capacity::<u8>(len);
        self.encode_raw(&mut buf);  // ArenaVec<u8> implements BufMut!
        buf.freeze()
    }
}

/// Trait for decoding protobuf messages.
///
/// This trait is implemented by builder types - mutable, arena-allocated construction
/// helpers that accumulate data during decoding. Builders can be frozen into immutable
/// views after construction is complete.
///
/// # Lifetimes
///
/// The `'arena` lifetime represents the lifetime of the arena from which the
/// builder allocates data. Builders cannot outlive their arena.
///
/// # Examples
///
/// ```ignore
/// use defiant::{Decode, Arena};
///
/// let arena = Arena::new();
/// let builder = MyMessageBuilder::decode(bytes, &arena)?;
/// let view = builder.freeze(); // convert to immutable view
/// ```
pub trait Decode<'arena>: Sized + 'arena {
    /// Creates a new empty builder initialized with the arena.
    ///
    /// This is used internally during decoding to create a builder that can
    /// accumulate repeated fields using arena-allocated storage.
    ///
    /// Meant to be used only by `Decode` implementations.
    #[doc(hidden)]
    fn new_in(arena: &'arena Arena) -> Self;

    /// Decodes a field from a buffer, and merges it into `self`.
    ///
    /// The arena is used to allocate any variable-length data (strings, bytes,
    /// repeated fields, etc.).
    ///
    /// Meant to be used only by `Decode` implementations.
    #[doc(hidden)]
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;

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
}

/// Links a view type to its corresponding builder type.
///
/// This trait associates an immutable view (which implements `Encode`) with
/// its mutable builder (which implements `Decode`).
pub trait MessageView<'arena>: Sized {
    /// The builder type for constructing this view
    type Builder: Decode<'arena>;

    /// Constructs a View from encoded bytes
    fn from_buf(buf: impl bytes::Buf, arena: &'arena Arena) -> Result<Self, DecodeError>;
}
