#![no_main]

use defiant::Arena;
use libfuzzer_sys::fuzz_target;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let arena = Arena::new();
    let _ = roundtrip::<TestAllTypesProto3>(data, &arena).unwrap_error();
});
