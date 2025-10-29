include!(concat!(env!("OUT_DIR"), "/default_string_escape.rs"));

#[test]
fn test_default_string_escape() {
    let arena = defiant::Arena::new();
    let builder = PersonBuilder::new_in(&arena);
    let msg = builder.freeze();
    assert_eq!(msg.name, r#"["unknown"]"#);
}
