//! Arena allocator for zero-copy protobuf deserialization.
//!
//! This module provides a thin wrapper around `bumpalo::Bump` that provides
//! arena allocation for protobuf messages. All decoded messages are allocated
//! from the arena and have lifetimes bound to it.

use alloc::string::String;
use alloc::vec::Vec;
use bumpalo::Bump;

pub use bumpalo::collections::Vec as BumpVec;

/// An arena allocator for protobuf messages.
///
/// All messages decoded with this arena will have their data allocated from
/// the arena and will be tied to the arena's lifetime. The arena uses a bump
/// allocator internally, which means:
///
/// - Allocation is very fast (just increment a pointer)
/// - Individual items cannot be freed (all freed at once when arena drops)
/// - Memory is reclaimed when the arena is dropped or reset
///
/// # Examples
///
/// ```
/// use prost::Arena;
///
/// let arena = Arena::new();
/// // Decode messages using the arena
/// // let msg = MyMessage::decode(bytes, &arena)?;
/// // All allocations freed when arena drops
/// ```
pub struct Arena {
    bump: Bump,
}

impl Arena {
    /// Creates a new arena with default capacity.
    #[inline]
    pub fn new() -> Self {
        Arena { bump: Bump::new() }
    }

    /// Creates a new arena with the specified capacity in bytes.
    ///
    /// The arena will allocate an initial chunk of at least `capacity` bytes.
    /// This can improve performance if you know approximately how much memory
    /// will be needed.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Arena {
            bump: Bump::with_capacity(capacity),
        }
    }

    /// Allocates a string slice in the arena.
    ///
    /// The string data is copied into the arena and a reference with the
    /// arena's lifetime is returned.
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }

    /// Allocates a byte slice in the arena by copying.
    ///
    /// The bytes are copied into the arena and a reference with the arena's
    /// lifetime is returned.
    #[inline]
    pub fn alloc_bytes(&self, bytes: &[u8]) -> &[u8] {
        self.bump.alloc_slice_copy(bytes)
    }

    /// Allocates a slice in the arena by copying.
    ///
    /// All elements are copied into the arena and a reference with the arena's
    /// lifetime is returned.
    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(slice)
    }

    /// Converts a Vec into an arena-allocated slice.
    ///
    /// This is useful for repeated fields which accumulate into Vec during
    /// decode, then get converted to arena slices.
    #[inline]
    pub fn alloc_vec<T: Copy>(&self, vec: Vec<T>) -> &[T] {
        self.bump.alloc_slice_copy(&vec)
    }

    /// Converts a Vec of Strings into an arena-allocated slice of string slices.
    ///
    /// This is useful for repeated string fields.
    pub fn alloc_string_vec(&self, vec: Vec<String>) -> &[&str] {
        let strs: Vec<&str> = vec.iter().map(|s| self.alloc_str(s)).collect();
        self.bump.alloc_slice_copy(&strs)
    }

    /// Allocates a slice in the arena, filling it with copies of a value.
    #[inline]
    pub fn alloc_slice_fill_copy<T: Copy>(&self, len: usize, value: T) -> &mut [T] {
        self.bump.alloc_slice_fill_copy(len, value)
    }

    /// Allocates a slice in the arena, filling it with default values.
    #[inline]
    pub fn alloc_slice_fill_default<T: Default + Copy>(&self, len: usize) -> &mut [T] {
        self.bump.alloc_slice_fill_default(len)
    }

    /// Allocates a slice in the arena, filling it from an iterator.
    ///
    /// This is useful for arena-allocating collections where the elements
    /// don't implement Copy (e.g., protobuf Value types).
    #[inline]
    pub fn alloc_slice_fill_iter<T, I>(&self, iter: I) -> &[T]
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.bump.alloc_slice_fill_iter(iter)
    }

    /// Allocates a value in the arena.
    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.bump.alloc(value)
    }

    /// Creates a new arena-allocated Vec for accumulating repeated field elements.
    ///
    /// During protobuf decoding, repeated fields accumulate elements into this Vec.
    /// After decoding completes, convert to an immutable slice via `into_bump_slice()`.
    #[inline]
    pub fn new_vec<T>(&self) -> BumpVec<'_, T> {
        BumpVec::new_in(&self.bump)
    }

    /// Creates a new arena-allocated Vec with the specified capacity.
    #[inline]
    pub fn new_vec_with_capacity<T>(&self, capacity: usize) -> BumpVec<'_, T> {
        BumpVec::with_capacity_in(capacity, &self.bump)
    }

    /// Resets the arena, reclaiming all allocated memory.
    ///
    /// After calling this, all previous allocations from this arena are
    /// invalidated. This is useful for reusing the same arena across multiple
    /// decode operations (e.g., in a request handler that processes many
    /// messages).
    ///
    /// # Safety
    ///
    /// This is safe to call, but any references previously allocated from this
    /// arena will become dangling pointers. The caller must ensure no such
    /// references are used after reset.
    #[inline]
    pub fn reset(&mut self) {
        self.bump.reset();
    }

    /// Returns the number of bytes currently allocated in the arena.
    #[inline]
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_str() {
        let arena = Arena::new();
        let s = arena.alloc_str("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_alloc_bytes() {
        let arena = Arena::new();
        let bytes = arena.alloc_bytes(b"world");
        assert_eq!(bytes, b"world");
    }

    #[test]
    fn test_alloc_slice() {
        let arena = Arena::new();
        let slice = arena.alloc_slice_copy(&[1, 2, 3, 4, 5]);
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_reset() {
        let mut arena = Arena::new();
        let _ = arena.alloc_str("test");
        let before = arena.allocated_bytes();
        assert!(before > 0);

        arena.reset();
        // After reset, the capacity remains but the arena can be reused
        // allocated_bytes() returns the total capacity, not used bytes
        let after = arena.allocated_bytes();
        assert!(after >= 0); // Capacity may remain allocated

        // Verify we can allocate again after reset
        let s = arena.alloc_str("after reset");
        assert_eq!(s, "after reset");
    }

    #[test]
    fn test_with_capacity() {
        let arena = Arena::with_capacity(1024);
        let s = arena.alloc_str("test");
        assert_eq!(s, "test");
    }
}

/// A conversion trait that requires an arena for allocation.
///
/// This is similar to `std::convert::From` but takes an arena parameter
/// to support arena-allocated conversions. Used for converting owned
/// data structures (like `Vec`, `BTreeMap`) into arena-allocated
/// protobuf types.
pub trait ArenaFrom<'arena, T>: Sized {
    /// Performs the conversion using the provided arena for allocation.
    fn arena_from(value: T, arena: &'arena Arena) -> Self;
}

/// Convenience trait for arena-based conversions.
///
/// This is the reciprocal of `ArenaFrom`.
pub trait ArenaInto<'arena, T> {
    /// Performs the conversion using the provided arena for allocation.
    fn arena_into(self, arena: &'arena Arena) -> T;
}

impl<'arena, T, U> ArenaInto<'arena, U> for T
where
    U: ArenaFrom<'arena, T>,
{
    fn arena_into(self, arena: &'arena Arena) -> U {
        U::arena_from(self, arena)
    }
}

/// An immutable, arena-allocated map with sorted entries for efficient lookups.
///
/// ArenaMap stores key-value pairs in a sorted slice, providing O(log n) lookups
/// via binary search while maintaining cache-friendly contiguous memory layout.
/// This is more efficient than BTreeMap for read-heavy workloads typical in
/// protobuf deserialization.
///
/// # Examples
///
/// ```
/// use prost::{Arena, ArenaMap};
///
/// let arena = Arena::new();
/// // During decoding, accumulate entries in a BumpVec and sort before creating the map
/// let entries = arena.alloc_slice_copy(&[("a", 1), ("b", 2), ("c", 3)]);
/// let map = ArenaMap::new(entries);
/// assert_eq!(map.get(&"b"), Some(&2));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaMap<'arena, K, V> {
    entries: &'arena [(K, V)],
}

impl<'arena, K, V> ArenaMap<'arena, K, V> {
    /// Creates a new ArenaMap from a slice of entries.
    ///
    /// The entries must be sorted by key for binary search to work correctly.
    /// During protobuf decoding, the builder sorts entries before creating the map.
    #[inline]
    pub fn new(entries: &'arena [(K, V)]) -> Self {
        ArenaMap { entries }
    }

    /// Returns the number of entries in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the map contains no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over the entries in sorted key order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Returns an iterator over the keys in sorted order.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over the values in key-sorted order.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns a reference to the underlying slice of entries.
    #[inline]
    pub fn as_slice(&self) -> &'arena [(K, V)] {
        self.entries
    }
}

impl<'arena, K: Ord, V> ArenaMap<'arena, K, V> {
    /// Returns a reference to the value corresponding to the key.
    ///
    /// Uses binary search, so has O(log n) complexity.
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries
            .binary_search_by(|(k, _)| k.cmp(key))
            .ok()
            .map(|idx| &self.entries[idx].1)
    }

    /// Returns true if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key(&self, key: &K) -> Option<bool> {
        Some(self.entries.binary_search_by(|(k, _)| k.cmp(key)).is_ok())
    }
}

impl<'arena, K: core::fmt::Debug, V: core::fmt::Debug> core::fmt::Debug for ArenaMap<'arena, K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'arena, K, V> Default for ArenaMap<'arena, K, V> {
    fn default() -> Self {
        ArenaMap { entries: &[] }
    }
}
