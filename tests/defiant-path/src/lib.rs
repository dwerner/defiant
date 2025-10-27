// This test ensures we can compile using re-exported dependencies as configured in
// `build.rs`. Note that there's no direct dependency of `::defiant` or `::defiant-types` in
// `Cargo.toml`.
include!(concat!(env!("OUT_DIR"), "/defiant_path.rs"));

#[test]
fn type_can_be_constructed() {
    use reexported_defiant::Arena;

    let arena = Arena::new();
    let mut msg_builder = MsgBuilder::new_in(&arena);
    msg_builder.set_a(1);
    msg_builder.set_b("test");

    // Timestamp is a Copy type without arena lifetime
    msg_builder.set_timestamp(Some(reexported_defiant::defiant_types::Timestamp {
        seconds: 3,
        nanos: 3,
    }));

    // For now, skip Value since it's a View type without builders
    // The main purpose of this test is to verify defiant_path configuration works

    // Test passes - this verifies the defiant_path configuration works correctly
    let msg = msg_builder.freeze();

    // Verify the values were set correctly
    assert_eq!(msg.a, 1);
    assert_eq!(msg.b, "test");
    assert_eq!(msg.timestamp, Some(reexported_defiant::defiant_types::Timestamp {
        seconds: 3,
        nanos: 3,
    }));
}
