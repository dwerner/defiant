// Ident conversion tests - using View API
use crate::roundtrip;
use defiant::{Arena, Encode};

pub mod bar_baz {
    include!(concat!(env!("OUT_DIR"), "/ident_conversion.bar_baz.rs"));
}

#[test]
fn test_ident_conversions() {
    let arena = Arena::new();

    // Create a FuzzBuster in the arena
    let fuzz_buster = bar_baz::foo_bar_baz::FuzzBuster {
        t: &[], // Empty map for now (maps are arena slices)
        nested_self: None,
    };
    let fuzz_buster_ref = arena.alloc(fuzz_buster);

    // Allocate the slice in the arena
    let fuzz_busters_slice = arena.alloc([fuzz_buster_ref]);

    let msg = bar_baz::FooBarBaz {
        foo_bar_baz: 42,
        fuzz_busters: fuzz_busters_slice,
        p_i_e: 0,
        r#as: 4,
        r#break: 5,
        r#const: 6,
        r#continue: 7,
        r#else: 8,
        r#enum: 9,
        r#false: 10,
        r#fn: 11,
        r#for: 12,
        r#if: 13,
        r#impl: 14,
        r#in: 15,
        r#let: 16,
        r#loop: 17,
        r#match: 18,
        r#mod: 19,
        r#move: 20,
        r#mut: 21,
        r#pub: 22,
        r#ref: 23,
        r#return: 24,
        r#static: 25,
        r#struct: 26,
        r#trait: 27,
        r#true: 28,
        r#type: 29,
        r#unsafe: 30,
        r#use: 31,
        r#where: 32,
        r#while: 33,
        r#dyn: 34,
        r#abstract: 35,
        r#become: 36,
        r#box: 37,
        r#do: 38,
        r#final: 39,
        r#macro: 40,
        r#override: 41,
        r#priv: 42,
        r#typeof: 43,
        r#unsized: 44,
        r#virtual: 45,
        r#yield: 46,
        r#async: 47,
        r#await: 48,
        r#try: 49,
        self_: 50,
        super_: 51,
        extern_: 52,
        crate_: 53,
        r#gen: 54,
    };

    let _ = bar_baz::foo_bar_baz::Self_ {};

    // Test enum ident conversion.
    let _ = bar_baz::foo_bar_baz::StrawberryRhubarbPie::Foo;
    let _ = bar_baz::foo_bar_baz::StrawberryRhubarbPie::Bar;
    let _ = bar_baz::foo_bar_baz::StrawberryRhubarbPie::FooBar;
    let _ = bar_baz::foo_bar_baz::StrawberryRhubarbPie::FuzzBuster;
    let _ = bar_baz::foo_bar_baz::StrawberryRhubarbPie::NormalRustEnumCase;

    let mut buf = alloc::vec::Vec::new();
    msg.encode(&mut buf);
    roundtrip::<bar_baz::FooBarBaz>(&buf, &arena).unwrap();
}
