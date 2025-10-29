include!(concat!(env!("OUT_DIR"), "/_.rs"));

#[test]
fn test_submessage_without_package() {
    let arena = defiant::Arena::new();
    let builder = MBuilder::new_in(&arena);
    let _msg = builder.freeze();
}
