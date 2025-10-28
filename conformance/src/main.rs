use std::io::{self, Read, Write};

use bytes::{Buf, BufMut};
use defiant::Encode;

use protobuf::conformance::{
    conformance_request, conformance_response, ConformanceRequest, WireFormat,
};
use protobuf::test_messages::proto2::TestAllTypesProto2;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::{roundtrip, RoundtripResult};

fn main() -> io::Result<()> {
    env_logger::init();
    let mut bytes = vec![0; 4];
    let mut arena = defiant::Arena::new();

    loop {
        bytes.resize(4, 0);

        if io::stdin().read_exact(&mut bytes).is_err() {
            // No more test cases.
            return Ok(());
        }

        let len = bytes.as_slice().get_u32_le() as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes)?;

        let result = match ConformanceRequest::from_buf(bytes.as_slice(), &arena) {
            Ok(request) => handle_request(&arena, request),
            Err(error) => {
                let error_str = arena.alloc_str(&format!("{error:?}"));
                conformance_response::Result::ParseError(error_str)
            }
        };

        let mut response_builder =
            protobuf::conformance::ConformanceResponseBuilder::new_in(&arena);
        response_builder.set_result(Some(result));
        let response = response_builder.freeze();

        let len = response.encoded_len();
        bytes.clear();
        bytes.put_u32_le(len as u32);
        response.encode(&mut bytes)?;
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes)?;
        stdout.flush()?;

        arena.reset();
    }
}

fn handle_request<'arena>(
    arena: &'arena defiant::Arena,
    request: ConformanceRequest<'arena>,
) -> conformance_response::Result<'arena> {
    let format =
        WireFormat::try_from(request.requested_output_format).unwrap_or(WireFormat::Unspecified);
    match format {
        WireFormat::Unspecified => {
            return conformance_response::Result::ParseError(
                arena.alloc_str("output format unspecified"),
            );
        }
        WireFormat::Json => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("JSON output is not supported"),
            );
        }
        WireFormat::Jspb => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("JSPB output is not supported"),
            );
        }
        WireFormat::TextFormat => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("TEXT_FORMAT output is not supported"),
            );
        }
        WireFormat::Protobuf => (),
    };

    let buf = match request.payload {
        None => return conformance_response::Result::ParseError(arena.alloc_str("no payload")),
        Some(conformance_request::Payload::JsonPayload(_)) => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("JSON input is not supported"),
            );
        }
        Some(conformance_request::Payload::JspbPayload(_)) => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("JSPB input is not supported"),
            );
        }
        Some(conformance_request::Payload::TextPayload(_)) => {
            return conformance_response::Result::Skipped(
                arena.alloc_str("TEXT input is not supported"),
            );
        }
        Some(conformance_request::Payload::ProtobufPayload(buf)) => buf,
    };

    let roundtrip = match request.message_type {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => {
            roundtrip::<TestAllTypesProto2>(&buf, arena)
        }
        "protobuf_test_messages.proto3.TestAllTypesProto3" => {
            roundtrip::<TestAllTypesProto3>(&buf, arena)
        }
        _ => {
            let error = arena.alloc_str(&format!("unknown message type: {}", request.message_type));
            return conformance_response::Result::ParseError(error);
        }
    };

    match roundtrip {
        RoundtripResult::Ok(buf) => {
            // Copy Vec<u8> into arena-allocated slice
            let mut arena_vec = arena.new_vec_with_capacity(buf.len());
            arena_vec.extend_from_slice(&buf);
            let buf_slice = arena_vec.freeze();
            conformance_response::Result::ProtobufPayload(buf_slice)
        }
        RoundtripResult::DecodeError(error) => {
            let error_str = arena.alloc_str(&error.to_string());
            conformance_response::Result::ParseError(error_str)
        }
        RoundtripResult::Error(error) => {
            let error_str = arena.alloc_str(&error.to_string());
            conformance_response::Result::RuntimeError(error_str)
        }
    }
}
