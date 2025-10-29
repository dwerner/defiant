include!(concat!(env!("OUT_DIR"), "/proto3_presence.rs"));

#[test]
fn test_proto3_presence() {
    use defiant::Encode;
    let arena = defiant::Arena::new();
    let mut builder = ABuilder::new_in(&arena);
    builder.set_b(42);
    builder.set_foo(Some(a::Foo::C(13)));
    let msg = builder.freeze();

    // Scalar-only messages don't implement MessageView, so just test basic encoding
    assert_eq!(msg.b, Some(42));
    assert_eq!(msg.foo, Some(a::Foo::C(13)));

    // Test encoding round-trip
    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();
    assert!(!buf.is_empty());
}
