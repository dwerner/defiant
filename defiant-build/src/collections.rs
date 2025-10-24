/// The map collection type to output for Protobuf `map` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum MapType {
    /// The [`std::collections::HashMap`] type.
    #[default]
    HashMap,
    /// The [`std::collections::BTreeMap`] type.
    BTreeMap,
}

/// The bytes collection type to output for Protobuf `bytes` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum BytesType {
    /// The [`defiant::alloc::vec::Vec<u8>`] type.
    #[default]
    Vec,
    /// The [`bytes::Bytes`](defiant::bytes::Bytes) type.
    Bytes,
}

impl MapType {
    /// The `prost-derive` annotation type corresponding to the map type.
    pub fn annotation(&self) -> &'static str {
        match self {
            MapType::HashMap => "map",
            MapType::BTreeMap => "btree_map",
        }
    }

    /// The fully-qualified Rust type corresponding to the map type.
    pub fn rust_type(&self) -> &'static str {
        match self {
            MapType::HashMap => "::std::collections::HashMap",
            MapType::BTreeMap => "::defiant::alloc::collections::BTreeMap",
        }
    }
}

impl BytesType {
    /// The `prost-derive` annotation type corresponding to the bytes type.
    pub fn annotation(&self) -> &'static str {
        match self {
            BytesType::Vec => "vec",
            BytesType::Bytes => "bytes",
        }
    }

    /// The fully-qualified Rust type corresponding to the bytes type.
    /// For arena-based messages, returns arena-allocated slices instead of owned types.
    pub fn rust_type(&self) -> &'static str {
        match self {
            BytesType::Vec => "&'arena [u8]",
            BytesType::Bytes => "&'arena [u8]",
        }
    }
}
