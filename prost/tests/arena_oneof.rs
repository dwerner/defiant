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

use prost::{Arena, DecodeError, Message};
use prost::encoding::{DecodeContext, WireType, string, int32, message};
use bytes::{Buf, BufMut};

/// Image message
#[derive(Debug, Clone)]
struct Image<'arena> {
    url: &'arena str,
    width: i32,
    height: i32,
}

impl<'arena> Default for Image<'arena> {
    fn default() -> Self {
        Image {
            url: "",
            width: 0,
            height: 0,
        }
    }
}

impl<'arena> Message<'arena> for Image<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Image {
            url: "",
            width: 0,
            height: 0,
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.url.is_empty() {
            string::encode(1, &self.url.to_string(), buf);
        }
        if self.width != 0 {
            int32::encode(2, &self.width, buf);
        }
        if self.height != 0 {
            int32::encode(3, &self.height, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                self.url = string::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            }
            2 => int32::merge(wire_type, &mut self.width, buf, ctx),
            3 => int32::merge(wire_type, &mut self.height, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !self.url.is_empty() {
            len += string::encoded_len(1, &self.url.to_string());
        }
        if self.width != 0 {
            len += int32::encoded_len(2, &self.width);
        }
        if self.height != 0 {
            len += int32::encoded_len(3, &self.height);
        }
        len
    }

    fn clear(&mut self) {
        self.url = "";
        self.width = 0;
        self.height = 0;
    }
}

/// Oneof enum - holds values directly
#[derive(Debug)]
enum Payload<'arena> {
    Text(&'arena str),
    Image(Image<'arena>),
    Count(i32),
}

/// Notification with oneof field
#[derive(Debug)]
struct Notification<'arena> {
    payload: Option<Payload<'arena>>,
}

impl<'arena> Default for Notification<'arena> {
    fn default() -> Self {
        Notification { payload: None }
    }
}

impl<'arena> Message<'arena> for Notification<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Notification { payload: None }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        match &self.payload {
            Some(Payload::Text(text)) => {
                string::encode(1, &text.to_string(), buf);
            }
            Some(Payload::Image(image)) => {
                message::encode(2, image, buf);
            }
            Some(Payload::Count(count)) => {
                int32::encode(3, count, buf);
            }
            None => {}
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        arena: &'arena Arena,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            1 => {
                // Decode text variant
                let text = string::merge_arena(wire_type, buf, arena, ctx)?;
                self.payload = Some(Payload::Text(text));
                Ok(())
            }
            2 => {
                // Decode image variant
                let mut image = Image::default();
                message::merge(wire_type, &mut image, buf, arena, ctx)?;
                self.payload = Some(Payload::Image(image));
                Ok(())
            }
            3 => {
                // Decode count variant
                let mut count = 0;
                int32::merge(wire_type, &mut count, buf, ctx)?;
                self.payload = Some(Payload::Count(count));
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        match &self.payload {
            Some(Payload::Text(text)) => string::encoded_len(1, &text.to_string()),
            Some(Payload::Image(image)) => message::encoded_len(2, image),
            Some(Payload::Count(count)) => int32::encoded_len(3, count),
            None => 0,
        }
    }

    fn clear(&mut self) {
        self.payload = None;
    }
}

#[test]
fn test_oneof_text() {
    let arena = Arena::new();

    let notification = Notification {
        payload: Some(Payload::Text("Hello, world!")),
    };

    let encoded = notification.encode_to_vec();
    println!("Encoded text variant: {} bytes", encoded.len());

    let decoded = Notification::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

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

    let decoded = Notification::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

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
    let decoded = Notification::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

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
    let decoded = Notification::decode(encoded.as_slice(), &arena)
        .expect("Failed to decode");

    assert!(decoded.payload.is_none());
}

#[test]
fn test_oneof_last_wins() {
    // In protobuf, if multiple oneof fields are set in the wire format,
    // the last one wins
    let arena = Arena::new();

    // Manually construct a message with multiple oneof fields set
    let mut buf = Vec::new();
    string::encode(1, &"first".to_string(), &mut buf);  // text
    int32::encode(3, &100, &mut buf);  // count

    let decoded = Notification::decode(buf.as_slice(), &arena)
        .expect("Failed to decode");

    // The last field (count) should win
    match decoded.payload {
        Some(Payload::Count(count)) => {
            assert_eq!(count, 100);
        }
        _ => panic!("Expected Count variant (last wins)"),
    }

    println!("Successfully verified oneof last-wins semantics");
}
