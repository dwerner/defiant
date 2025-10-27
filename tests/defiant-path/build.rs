fn main() {
    let arena = defiant::Arena::new();

    defiant_build::Config::new(&arena)
        .defiant_path("::reexported_defiant::defiant")
        .defiant_types_path("::reexported_defiant::defiant_types")
        .compile_protos(&["protos/defiant_path.proto"], &["protos"])
        .unwrap();
}
