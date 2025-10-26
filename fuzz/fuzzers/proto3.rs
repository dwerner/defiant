#![no_main]

use libfuzzer_sys::fuzz_target;
use defiant::Arena;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let arena = Arena::new();
    let _ = roundtrip::<TestAllTypesProto3>(data, &arena).unwrap_error();
});
