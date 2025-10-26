// This test ensures we can compile using re-exported dependencies as configured in
// `build.rs`. Note that there's no direct dependency of `::defiant` or `::defiant-types` in
// `Cargo.toml`.
include!(concat!(env!("OUT_DIR"), "/defiant_path.rs"));

#[test]
fn type_can_be_constructed() {
    use reexported_defiant::defiant_types::value::Kind;
    use reexported_defiant::defiant_types::{Timestamp, Value};
    use reexported_defiant::Arena;

    use self::msg::C;

    let arena = Arena::new();
    let mut msg_builder = MsgBuilder::new_in(&arena);
    msg_builder.set_a(1);
    msg_builder.set_b("test");

    let mut timestamp_builder = TimestampBuilder::new();
    timestamp_builder.set_nanos(3);
    timestamp_builder.set_seconds(3);
    msg_builder.set_timestamp(Some(timestamp_builder.freeze()));

    let mut value_builder = ValueBuilder::new();
    value_builder.set_kind(Some(Kind::BoolValue(true)));
    msg_builder.set_value(Some(value_builder.freeze()));

    msg_builder.set_c(Some(C::D(1)));

    let _msg = msg_builder.freeze();
}
