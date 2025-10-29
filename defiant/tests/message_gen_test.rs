//! Test to see what the derive macro generates

use defiant_derive::View;

#[derive(View)]
struct SimplePerson<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(int32, tag = "2")]
    age: i32,
}

#[derive(View)]
struct PersonWithRepeated<'arena> {
    #[defiant(string, tag = "1")]
    name: &'arena str,

    #[defiant(string, repeated, tag = "2")]
    tags: &'arena [&'arena str],
}

#[test]
fn test_message_struct_generated() {
    use defiant::{Arena, MessageView};

    let arena = Arena::new();

    // Test using the Builder associated type
    let mut msg = <PersonWithRepeated as MessageView>::Builder::new_in(&arena);

    // Test setters
    msg.set_name("Alice");
    msg.push_tags("rust");
    msg.push_tags("protobuf");

    // Test getters
    assert_eq!(msg.name(), "Alice");
    assert_eq!(msg.tags(), &["rust", "protobuf"]);

    // Test getters - can inspect while building
    assert_eq!(msg.name(), "Alice");
    assert_eq!(msg.tags(), &["rust", "protobuf"]);

    // Can continue modifying
    msg.push_tags("arena");
    assert_eq!(msg.tags(), &["rust", "protobuf", "arena"]);

    // Test into_view() - consumes builder, returns arena-lifetime view
    let final_view = msg.freeze();
    assert_eq!(final_view.name, "Alice");
    assert_eq!(final_view.tags, &["rust", "protobuf", "arena"]);
}
