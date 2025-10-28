include!(concat!(env!("OUT_DIR"), "/proto3_presence.rs"));

#[test]
fn test_proto3_presence() {
    let arena = defiant::Arena::new();
    let mut builder = ABuilder::new_in(&arena);
    builder.set_b(Some(42));
    builder.set_foo(Some(a::Foo::C(13)));
    let msg = builder.freeze();

    crate::check_message(&msg, &arena);
}
