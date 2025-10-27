//! Test to see what the derive macro generates

use defiant_derive::Message;

#[derive(Message)]
struct SimplePerson<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(int32, tag = "2")]
    age: i32,
}

#[derive(Message)]
struct PersonWithRepeated<'arena> {
    #[prost(string, tag = "1")]
    name: &'arena str,

    #[prost(string, repeated, tag = "2")]
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
