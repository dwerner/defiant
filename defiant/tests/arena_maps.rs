//! Test for arena-allocated map fields
//!
//! This test demonstrates map fields with arena allocation.
//! The corresponding proto would be:
//!
//! ```proto
//! message UserProfile {
//!   string username = 1;
//!   map<string, string> metadata = 2;
//!   map<int32, string> tags = 3;
//! }
//! ```
//!
//! In protobuf wire format, maps are encoded as repeated messages:
//! ```proto
//! message MetadataEntry {
//!   string key = 1;
//!   string value = 2;
//! }
//! repeated MetadataEntry metadata = 2;
//! ```

use defiant::{Arena, DecodeError, Message};
use defiant::encoding::{DecodeContext, WireType, string, int32, message};
use bytes::{Buf, BufMut};

/// Helper struct for decoding string->string map entries
#[derive(Debug, Default, Clone)]
struct StringMapEntry<'arena> {
    key: &'arena str,
    value: &'arena str,
}

impl<'arena> Message<'arena> for StringMapEntry<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        StringMapEntry {
            key: "",
            value: "",
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.key.is_empty() {
            string::encode(1, &self.key.to_string(), buf);
        }
        if !self.value.is_empty() {
            string::encode(2, &self.value.to_string(), buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                self.key = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            2 => {
                self.value = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            _ => {
                defiant::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.key.is_empty() {
            len += string::encoded_len(1, &self.key.to_string());
        }
        if !self.value.is_empty() {
            len += string::encoded_len(2, &self.value.to_string());
        }
        len
    }

}

/// Helper struct for decoding int32->string map entries
#[derive(Debug, Default, Clone)]
struct IntStringMapEntry<'arena> {
    key: i32,
    value: &'arena str,
}

impl<'arena> Message<'arena> for IntStringMapEntry<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        IntStringMapEntry {
            key: 0,
            value: "",
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if self.key != 0 {
            int32::encode(1, &self.key, buf);
        }
        if !self.value.is_empty() {
            string::encode(2, &self.value.to_string(), buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                int32::merge(wire_type, &mut self.key, buf, ctx)
            }
            2 => {
                self.value = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            _ => {
                defiant::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if self.key != 0 {
            len += int32::encoded_len(1, &self.key);
        }
        if !self.value.is_empty() {
            len += string::encoded_len(2, &self.value.to_string());
        }
        len
    }

}

/// Builder for UserProfile (accumulates during decode)
#[derive(Debug, Default)]
struct UserProfileBuilder {
    username: String,
    metadata: Vec<StringMapEntry<'static>>,
    tags: Vec<IntStringMapEntry<'static>>,
}

impl<'arena> Message<'arena> for UserProfileBuilder {
    fn new_in(_arena: &'arena Arena) -> Self {
        UserProfileBuilder {
            username: String::new(),
            metadata: Vec::new(),
            tags: Vec::new(),
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.username.is_empty() {
            string::encode(1, self.username.as_str(), buf);
        }
        for entry in &self.metadata {
            message::encode(2, entry, buf);
        }
        for entry in &self.tags {
            message::encode(3, entry, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                // Inline string decoding for owned String
                use defiant::encoding::{check_wire_type, decode_varint, WireType};
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)? as usize;
                self.username.clear();
                self.username.reserve(len);
                unsafe {
                    self.username.as_mut_vec().resize(len, 0);
                    buf.copy_to_slice(self.username.as_mut_vec());
                }
                if !std::str::from_utf8(self.username.as_bytes()).is_ok() {
                    return Err(defiant::DecodeError::new("invalid UTF-8"));
                }
                Ok(())
            }
            2 => {
                // Decode map entry
                let mut entry: StringMapEntry<'arena> = StringMapEntry::default();
                message::merge(wire_type, &mut entry, buf, arena, ctx)?;
                // SAFETY: We transmute to 'static temporarily, will convert to arena storage
                let entry_static: StringMapEntry<'static> = unsafe { std::mem::transmute(entry) };
                self.metadata.push(entry_static);
                Ok(())
            }
            3 => {
                // Decode map entry
                let mut entry: IntStringMapEntry<'arena> = IntStringMapEntry::default();
                message::merge(wire_type, &mut entry, buf, arena, ctx)?;
                // SAFETY: We transmute to 'static temporarily, will convert to arena storage
                let entry_static: IntStringMapEntry<'static> = unsafe { std::mem::transmute(entry) };
                self.tags.push(entry_static);
                Ok(())
            }
            _ => {
                defiant::encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.username.is_empty() {
            len += string::encoded_len(1, self.username.as_str());
        }
        for entry in &self.metadata {
            len += message::encoded_len(2, entry);
        }
        for entry in &self.tags {
            len += message::encoded_len(3, entry);
        }
        len
    }

}

/// Final UserProfile with arena-allocated maps
#[derive(Debug)]
struct UserProfile<'arena> {
    username: &'arena str,
    metadata: &'arena [(&'arena str, &'arena str)],
    tags: &'arena [(i32, &'arena str)],
}

impl UserProfileBuilder {
    fn into_arena<'arena>(self, arena: &'arena Arena) -> UserProfile<'arena> {
        // Convert metadata entries to tuples
        let metadata_tuples: Vec<(&str, &str)> = unsafe {
            std::mem::transmute::<Vec<StringMapEntry<'static>>, Vec<StringMapEntry<'arena>>>(self.metadata)
        }
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();

        // Convert tags entries to tuples
        let tags_tuples: Vec<(i32, &str)> = unsafe {
            std::mem::transmute::<Vec<IntStringMapEntry<'static>>, Vec<IntStringMapEntry<'arena>>>(self.tags)
        }
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();

        UserProfile {
            username: arena.alloc_str(&self.username),
            metadata: {
                let mut vec = arena.new_vec();
                vec.extend_from_slice(&metadata_tuples);
                vec.freeze()
            },
            tags: {
                let mut vec = arena.new_vec();
                vec.extend_from_slice(&tags_tuples);
                vec.freeze()
            },
        }
    }
}

#[test]
fn test_map_basic() {
    let arena = Arena::new();

    let builder = UserProfileBuilder {
        username: "alice".to_string(),
        metadata: vec![
            StringMapEntry { key: "email", value: "alice@example.com" },
            StringMapEntry { key: "role", value: "admin" },
        ],
        tags: vec![
            IntStringMapEntry { key: 1, value: "important" },
            IntStringMapEntry { key: 2, value: "verified" },
        ],
    };

    let encoded = builder.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    let decoded_builder = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let profile = decoded_builder.into_arena(&arena);

    assert_eq!(profile.username, "alice");
    assert_eq!(profile.metadata.len(), 2);
    assert_eq!(profile.tags.len(), 2);

    // Check metadata map entries
    assert_eq!(profile.metadata[0].0, "email");
    assert_eq!(profile.metadata[0].1, "alice@example.com");
    assert_eq!(profile.metadata[1].0, "role");
    assert_eq!(profile.metadata[1].1, "admin");

    // Check tags map entries
    assert_eq!(profile.tags[0].0, 1);
    assert_eq!(profile.tags[0].1, "important");
    assert_eq!(profile.tags[1].0, 2);
    assert_eq!(profile.tags[1].1, "verified");

    println!("Successfully decoded maps");
    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_map_empty() {
    let arena = Arena::new();

    let builder = UserProfileBuilder {
        username: "bob".to_string(),
        metadata: vec![],
        tags: vec![],
    };

    let encoded = builder.encode_to_vec();
    let decoded_builder = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let profile = decoded_builder.into_arena(&arena);

    assert_eq!(profile.username, "bob");
    assert_eq!(profile.metadata.len(), 0);
    assert_eq!(profile.tags.len(), 0);
}

#[test]
fn test_map_multiple_entries() {
    let arena = Arena::new();

    // Use actual static strings for the test
    let builder = UserProfileBuilder {
        username: "biguser".to_string(),
        metadata: vec![
            StringMapEntry { key: "email", value: "user@example.com" },
            StringMapEntry { key: "name", value: "Big User" },
            StringMapEntry { key: "role", value: "admin" },
            StringMapEntry { key: "department", value: "engineering" },
            StringMapEntry { key: "location", value: "office" },
        ],
        tags: vec![],
    };

    let encoded = builder.encode_to_vec();
    println!("Encoded {} map entries: {} bytes", 5, encoded.len());

    let decoded_builder = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let profile = decoded_builder.into_arena(&arena);

    assert_eq!(profile.username, "biguser");
    assert_eq!(profile.metadata.len(), 5);

    // Check entries
    assert_eq!(profile.metadata[0].0, "email");
    assert_eq!(profile.metadata[0].1, "user@example.com");
    assert_eq!(profile.metadata[2].0, "role");
    assert_eq!(profile.metadata[2].1, "admin");

    println!("Arena allocated {} bytes for {} map entries", arena.allocated_bytes(), profile.metadata.len());
}

#[test]
fn test_map_lookup() {
    let arena = Arena::new();

    let builder = UserProfileBuilder {
        username: "lookup_test".to_string(),
        metadata: vec![
            StringMapEntry { key: "a", value: "alpha" },
            StringMapEntry { key: "b", value: "beta" },
            StringMapEntry { key: "c", value: "gamma" },
        ],
        tags: vec![],
    };

    let encoded = builder.encode_to_vec();
    let decoded_builder = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    let profile = decoded_builder.into_arena(&arena);

    // User can build a HashMap from the slice if needed
    use std::collections::HashMap;
    let map: HashMap<&str, &str> = profile.metadata.iter().copied().collect();

    assert_eq!(map.get("a"), Some(&"alpha"));
    assert_eq!(map.get("b"), Some(&"beta"));
    assert_eq!(map.get("c"), Some(&"gamma"));
    assert_eq!(map.get("d"), None);

    println!("Successfully converted arena slice to HashMap");
}
