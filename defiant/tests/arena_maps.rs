//! Test for arena-allocated map fields

use defiant_derive::View;
use defiant::{Arena, ArenaMap, Encode};

/// UserProfile with map fields
#[derive(View)]
struct UserProfile<'arena> {
    #[defiant(string, tag = 1)]
    username: &'arena str,
    #[defiant(arena_map = "string, string", tag = 2)]
    metadata: ArenaMap<'arena, &'arena str, &'arena str>,
    #[defiant(arena_map = "int32, string", tag = 3)]
    tags: ArenaMap<'arena, i32, &'arena str>,
}

#[test]
fn test_map_basic() {
    let arena = Arena::new();

    let metadata_entries: &[(&str, &str)] = &[("email", "alice@example.com"), ("role", "admin")];
    let metadata = ArenaMap::new(metadata_entries);

    let tags_entries: &[(i32, &str)] = &[(1, "important"), (2, "verified")];
    let tags = ArenaMap::new(tags_entries);

    let profile = UserProfile {
        username: "alice",
        metadata,
        tags,
    };

    let encoded = profile.encode_to_vec();
    println!("Encoded {} bytes", encoded.len());

    let decoded = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.username, "alice");
    assert_eq!(decoded.metadata.len(), 2);
    assert_eq!(decoded.tags.len(), 2);

    // Check metadata map entries
    assert_eq!(decoded.metadata.get(&"email"), Some(&"alice@example.com"));
    assert_eq!(decoded.metadata.get(&"role"), Some(&"admin"));

    // Check tags map entries
    assert_eq!(decoded.tags.get(&1), Some(&"important"));
    assert_eq!(decoded.tags.get(&2), Some(&"verified"));

    println!("Successfully decoded maps");
    println!("Arena allocated {} bytes", arena.allocated_bytes());
}

#[test]
fn test_map_empty() {
    let arena = Arena::new();

    let profile = UserProfile {
        username: "bob",
        metadata: ArenaMap::new(&[]),
        tags: ArenaMap::new(&[]),
    };

    let encoded = profile.encode_to_vec();
    let decoded = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.username, "bob");
    assert_eq!(decoded.metadata.len(), 0);
    assert_eq!(decoded.tags.len(), 0);
}

#[test]
fn test_map_multiple_entries() {
    let arena = Arena::new();

    let metadata_entries: &[(&str, &str)] = &[
        ("department", "engineering"),
        ("email", "user@example.com"),
        ("location", "office"),
        ("name", "Big User"),
        ("role", "admin"),
    ];
    let metadata = ArenaMap::new(metadata_entries);

    let profile = UserProfile {
        username: "biguser",
        metadata,
        tags: ArenaMap::new(&[]),
    };

    let encoded = profile.encode_to_vec();
    println!("Encoded {} map entries: {} bytes", 5, encoded.len());

    let decoded = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.username, "biguser");
    assert_eq!(decoded.metadata.len(), 5);

    // Check entries
    assert_eq!(decoded.metadata.get(&"email"), Some(&"user@example.com"));
    assert_eq!(decoded.metadata.get(&"role"), Some(&"admin"));

    println!(
        "Arena allocated {} bytes for {} map entries",
        arena.allocated_bytes(),
        decoded.metadata.len()
    );
}

#[test]
fn test_map_lookup() {
    let arena = Arena::new();

    let metadata_entries: &[(&str, &str)] = &[("a", "alpha"), ("b", "beta"), ("c", "gamma")];
    let metadata = ArenaMap::new(metadata_entries);

    let profile = UserProfile {
        username: "lookup_test",
        metadata,
        tags: ArenaMap::new(&[]),
    };

    let encoded = profile.encode_to_vec();
    let decoded = UserProfileBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode")
        .freeze();

    assert_eq!(decoded.metadata.get(&"a"), Some(&"alpha"));
    assert_eq!(decoded.metadata.get(&"b"), Some(&"beta"));
    assert_eq!(decoded.metadata.get(&"c"), Some(&"gamma"));
    assert_eq!(decoded.metadata.get(&"d"), None);

    println!("Successfully tested map lookups");
}
