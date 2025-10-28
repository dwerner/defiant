#![cfg(ignore)]
// TODO: Migrate to View API
// TODO: View derive doesn't support generic type parameters yet
// This test is disabled until generic support is added

#[test]
#[ignore = "View derive doesn't support generics yet"]
fn generic_enum() {
    let msg = GenericMessage { data: Some(100u64) };
    let enumeration = GenericEnum::Data(msg);
    match enumeration {
        GenericEnum::Data(d) => assert_eq!(100, d.data.unwrap()),
        GenericEnum::Number(_) => panic!("Not supposed to reach"),
    }
}
