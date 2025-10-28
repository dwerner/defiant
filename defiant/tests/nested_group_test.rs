use defiant_derive::View;
use defiant::Arena;

#[derive(Clone, PartialEq, View)]
pub struct Outer<'arena> {
    #[defiant(group, repeated, tag = "1")]
    pub inner_group: &'arena [InnerGroup<'arena>],
}

#[derive(Clone, PartialEq, View)]
pub struct InnerGroup<'arena> {
    #[defiant(string, required, tag = "2")]
    pub name: &'arena str,
}

#[test]
fn test_nested_group() {
    use defiant::MessageView;

    let arena = Arena::new();
    // Create builder, then freeze to get view
    let builder = <Outer as MessageView>::Builder::new_in(&arena);
    let outer = builder.freeze();
    println!("Created outer: {:?}", outer);

    // Verify it's empty
    assert_eq!(outer.inner_group.len(), 0);
}
