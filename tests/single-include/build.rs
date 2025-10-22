use prost_build::Config;

fn main() {
    let arena = prost::Arena::new();

    Config::new(&arena)
        .include_file("lib.rs")
        .compile_protos(&["protos/search.proto"], &["protos"])
        .unwrap();

    Config::new(&arena)
        .out_dir("src/outdir")
        .include_file("mod.rs")
        .compile_protos(&["protos/outdir.proto"], &["protos"])
        .unwrap();
}
