use prost::{Arena, Message};

#[derive(Clone, PartialEq, Message)]
pub struct Outer<'arena> {
    #[prost(group, repeated, tag = "1")]
    pub inner_group: &'arena [InnerGroup<'arena>],
}

#[derive(Clone, PartialEq, Message)]
pub struct InnerGroup<'arena> {
    #[prost(string, required, tag = "2")]
    pub name: &'arena str,
}

#[test]
fn test_nested_group() {
    use prost::MessageView;

    let arena = Arena::new();
    // Create builder, then freeze to get view
    let builder = <Outer as MessageView>::Builder::new_in(&arena);
    let outer = builder.freeze();
    println!("Created outer: {:?}", outer);

    // Verify it's empty
    assert_eq!(outer.inner_group.len(), 0);
}
