//! Test for arena-allocated oneof fields
//!
//! This test demonstrates oneof fields with arena allocation.
//! The corresponding proto would be:
//!
//! ```proto
//! message Image {
//!   string url = 1;
//!   int32 width = 2;
//!   int32 height = 3;
//! }
//!
//! message Notification {
//!   oneof payload {
//!     string text = 1;
//!     Image image = 2;
//!     int32 count = 3;
//!   }
//! }
//! ```

use defiant::{Arena, Message, Oneof, Encode, Decode};

/// Image message
#[derive(Clone, PartialEq, Message)]
struct Image<'arena> {
    #[defiant(string, tag = 1)]
    url: &'arena str,
    #[defiant(int32, tag = 2)]
    width: i32,
    #[defiant(int32, tag = 3)]
    height: i32,
}

/// Notification with oneof field
#[derive(Message)]
struct Notification<'arena> {
    #[defiant(oneof = "Payload", tags = "1, 2, 3")]
    payload: Option<Payload<'arena>>,
}

/// Oneof enum - holds values directly
#[derive(Clone, PartialEq, Oneof)]
enum Payload<'arena> {
    #[defiant(string, tag = 1)]
    Text(&'arena str),
    #[defiant(message, tag = 2)]
    Image(Image<'arena>),
    #[defiant(int32, tag = 3)]
    Count(i32),
}

#[test]
fn test_oneof_text() {
    let arena = Arena::new();

    let notification = Notification {
        payload: Some(Payload::Text("Hello, world!")),
    };

    let encoded = notification.encode_to_vec();
    println!("Encoded text variant: {} bytes", encoded.len());

    let decoded = NotificationBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    match decoded.payload {
        Some(Payload::Text(text)) => {
            assert_eq!(text, "Hello, world!");
        }
        _ => panic!("Expected Text variant"),
    }

    println!("Successfully decoded oneof text variant");
}

#[test]
fn test_oneof_image() {
    let arena = Arena::new();

    let notification = Notification {
        payload: Some(Payload::Image(Image {
            url: "https://example.com/image.jpg",
            width: 1920,
            height: 1080,
        })),
    };

    let encoded = notification.encode_to_vec();
    println!("Encoded image variant: {} bytes", encoded.len());

    let decoded = NotificationBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    match decoded.payload {
        Some(Payload::Image(image)) => {
            assert_eq!(image.url, "https://example.com/image.jpg");
            assert_eq!(image.width, 1920);
            assert_eq!(image.height, 1080);
        }
        _ => panic!("Expected Image variant"),
    }

    println!("Successfully decoded oneof image variant");
}

#[test]
fn test_oneof_count() {
    let arena = Arena::new();

    let notification = Notification {
        payload: Some(Payload::Count(42)),
    };

    let encoded = notification.encode_to_vec();
    let decoded = NotificationBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    match decoded.payload {
        Some(Payload::Count(count)) => {
            assert_eq!(count, 42);
        }
        _ => panic!("Expected Count variant"),
    }
}

#[test]
fn test_oneof_none() {
    let arena = Arena::new();

    let notification = Notification { payload: None };

    let encoded = notification.encode_to_vec();
    let decoded = NotificationBuilder::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    assert!(decoded.payload.is_none());
}

#[test]
fn test_oneof_last_wins() {
    // In protobuf, if multiple oneof fields are set in the wire format,
    // the last one wins
    let arena = Arena::new();

    // Manually construct a message with multiple oneof fields set
    let mut buf = Vec::new();
    defiant::encoding::string::encode(1, "first", &mut buf);  // text
    defiant::encoding::int32::encode(3, &100, &mut buf);  // count

    let decoded = NotificationBuilder::decode(buf.as_slice(), &arena)
        .expect("Failed to decode").freeze();

    // The last field (count) should win
    match decoded.payload {
        Some(Payload::Count(count)) => {
            assert_eq!(count, 100);
        }
        _ => panic!("Expected Count variant (last wins)"),
    }

    println!("Successfully verified oneof last-wins semantics");
}
