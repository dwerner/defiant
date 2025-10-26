#![doc(html_root_url = "https://docs.rs/defiant/0.1.0")]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

// Allow tests to use `prost::` imports
#[doc(hidden)]
extern crate self as prost;

// Re-export the alloc crate for use within derived code.
#[doc(hidden)]
pub extern crate alloc;

// Re-export the bytes crate for use within derived code.
pub use bytes;

pub mod arena;
mod error;
mod message;
mod name;
mod types;

#[doc(hidden)]
pub mod encoding;

pub use crate::arena::{Arena, ArenaFrom, ArenaInto, ArenaMap, ArenaVec};
pub use crate::encoding::length_delimiter::{
    decode_length_delimiter, encode_length_delimiter, length_delimiter_len,
};
pub use crate::error::{DecodeError, EncodeError, UnknownEnumValue};
pub use crate::message::{Decode, Encode, MessageView};
pub use crate::name::Name;

// See `encoding::DecodeContext` for more info.
// 100 is the default recursion limit in the C++ implementation.
#[cfg(not(feature = "no-recursion-limit"))]
const RECURSION_LIMIT: u32 = 100;

// Re-export #[derive(Message, Enumeration, Oneof)].
// Based on serde's equivalent re-export [1], but enabled by default.
//
// [1]: https://github.com/serde-rs/serde/blob/v1.0.89/serde/src/lib.rs#L245-L256
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate defiant_derive;
#[cfg(feature = "derive")]
#[allow(unused_imports)]
extern crate defiant_derive as prost_derive;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use defiant_derive::*;
