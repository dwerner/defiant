fn main() {
    let arena = prost::Arena::new();

    prost_build::Config::new(&arena)
        .prost_path("::reexported_prost::prost")
        .prost_types_path("::reexported_prost::prost_types")
        .compile_protos(&["protos/prost_path.proto"], &["protos"])
        .unwrap();
}
