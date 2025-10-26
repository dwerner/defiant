//! Test optional message field handling

use defiant::{Arena, Encode};

#[derive(defiant_derive::Message)]
struct Inner<'arena> {
    #[defiant(string, tag = "1")]
    value: &'arena str,
}

#[derive(defiant_derive::Message)]
struct Outer<'arena> {
    #[defiant(message, optional, tag = "1")]
    inner: ::core::option::Option<&'arena Inner<'arena>>,
}

#[test]
fn test_optional_message() {
    let arena = Arena::new();
    let outer = OuterBuilder::new_in(&arena);
    assert!(outer.inner().is_none());
}
