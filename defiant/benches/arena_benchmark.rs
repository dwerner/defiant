use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use prost::{Arena, Message};

// Traditional owned message
#[derive(Debug, Default)]
struct PersonOwned {
    name: String,
    email: String,
    phone: String,
    address: String,
}

impl<'arena> Message<'arena> for PersonOwned {
    fn new_in(_arena: &'arena Arena) -> Self {
        Self::default()
    }

    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        if !self.name.is_empty() {
            prost::encoding::string::encode(1, &self.name, buf);
        }
        if !self.email.is_empty() {
            prost::encoding::string::encode(2, &self.email, buf);
        }
        if !self.phone.is_empty() {
            prost::encoding::string::encode(3, &self.phone, buf);
        }
        if !self.address.is_empty() {
            prost::encoding::string::encode(4, &self.address, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::wire_type::WireType,
        buf: &mut impl prost::bytes::Buf,
        _arena: &'arena Arena,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        use prost::encoding::{check_wire_type, decode_varint, WireType};

        match tag {
            1 => {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)? as usize;
                self.name.clear();
                self.name.reserve(len);
                unsafe {
                    self.name.as_mut_vec().resize(len, 0);
                    buf.copy_to_slice(self.name.as_mut_vec());
                }
                if !std::str::from_utf8(self.name.as_bytes()).is_ok() {
                    return Err(prost::DecodeError::new("invalid UTF-8"));
                }
                Ok(())
            }
            2 => {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)? as usize;
                self.email.clear();
                self.email.reserve(len);
                unsafe {
                    self.email.as_mut_vec().resize(len, 0);
                    buf.copy_to_slice(self.email.as_mut_vec());
                }
                if !std::str::from_utf8(self.email.as_bytes()).is_ok() {
                    return Err(prost::DecodeError::new("invalid UTF-8"));
                }
                Ok(())
            }
            3 => {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)? as usize;
                self.phone.clear();
                self.phone.reserve(len);
                unsafe {
                    self.phone.as_mut_vec().resize(len, 0);
                    buf.copy_to_slice(self.phone.as_mut_vec());
                }
                if !std::str::from_utf8(self.phone.as_bytes()).is_ok() {
                    return Err(prost::DecodeError::new("invalid UTF-8"));
                }
                Ok(())
            }
            4 => {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)? as usize;
                self.address.clear();
                self.address.reserve(len);
                unsafe {
                    self.address.as_mut_vec().resize(len, 0);
                    buf.copy_to_slice(self.address.as_mut_vec());
                }
                if !std::str::from_utf8(self.address.as_bytes()).is_ok() {
                    return Err(prost::DecodeError::new("invalid UTF-8"));
                }
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0
            + if !self.name.is_empty() {
                prost::encoding::string::encoded_len(1, &self.name)
            } else {
                0
            }
            + if !self.email.is_empty() {
                prost::encoding::string::encoded_len(2, &self.email)
            } else {
                0
            }
            + if !self.phone.is_empty() {
                prost::encoding::string::encoded_len(3, &self.phone)
            } else {
                0
            }
            + if !self.address.is_empty() {
                prost::encoding::string::encoded_len(4, &self.address)
            } else {
                0
            }
    }
}

// Arena-allocated message
#[derive(Debug)]
struct PersonArena<'arena> {
    name: &'arena str,
    email: &'arena str,
    phone: &'arena str,
    address: &'arena str,
}

impl<'arena> Default for PersonArena<'arena> {
    fn default() -> Self {
        PersonArena {
            name: "",
            email: "",
            phone: "",
            address: "",
        }
    }
}

impl<'arena> Message<'arena> for PersonArena<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Self::default()
    }

    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        if !self.name.is_empty() {
            prost::encoding::string::encode(1, &self.name.to_string(), buf);
        }
        if !self.email.is_empty() {
            prost::encoding::string::encode(2, &self.email.to_string(), buf);
        }
        if !self.phone.is_empty() {
            prost::encoding::string::encode(3, &self.phone.to_string(), buf);
        }
        if !self.address.is_empty() {
            prost::encoding::string::encode(4, &self.address.to_string(), buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::wire_type::WireType,
        buf: &mut impl prost::bytes::Buf,
        arena: &'arena Arena,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => prost::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.name = v),
            2 => prost::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.email = v),
            3 => prost::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.phone = v),
            4 => prost::encoding::string::merge_arena(wire_type, buf, arena, ctx)
                .map(|v| self.address = v),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0
            + if !self.name.is_empty() {
                prost::encoding::string::encoded_len(1, &self.name.to_string())
            } else {
                0
            }
            + if !self.email.is_empty() {
                prost::encoding::string::encoded_len(2, &self.email.to_string())
            } else {
                0
            }
            + if !self.phone.is_empty() {
                prost::encoding::string::encoded_len(3, &self.phone.to_string())
            } else {
                0
            }
            + if !self.address.is_empty() {
                prost::encoding::string::encoded_len(4, &self.address.to_string())
            } else {
                0
            }
    }
}

fn create_test_data() -> Vec<u8> {
    let mut data = Vec::new();
    // name = "Alice Johnson" (13 bytes)
    data.extend_from_slice(&[0x0a, 0x0d]);
    data.extend_from_slice(b"Alice Johnson");
    // email = "alice.johnson@example.com" (25 bytes)
    data.extend_from_slice(&[0x12, 0x19]);
    data.extend_from_slice(b"alice.johnson@example.com");
    // phone = "+1-555-0123" (11 bytes)
    data.extend_from_slice(&[0x1a, 0x0b]);
    data.extend_from_slice(b"+1-555-0123");
    // address = "123 Main Street, Portland, OR 97201" (35 bytes)
    data.extend_from_slice(&[0x22, 0x23]);
    data.extend_from_slice(b"123 Main Street, Portland, OR 97201");
    data
}

fn bench_decode_owned(c: &mut Criterion) {
    let data = create_test_data();

    c.bench_function("decode_owned", |b| {
        b.iter(|| {
            let arena = Arena::new();
            let msg = PersonOwned::decode(black_box(&data[..]), &arena).unwrap();
            black_box(msg);
        });
    });
}

fn bench_decode_arena(c: &mut Criterion) {
    let data = create_test_data();

    c.bench_function("decode_arena", |b| {
        b.iter(|| {
            let arena = Arena::new();
            let msg = PersonArena::decode(black_box(&data[..]), &arena).unwrap();
            black_box(msg);
        });
    });
}

fn bench_decode_batch_owned(c: &mut Criterion) {
    let data = create_test_data();

    c.bench_function("decode_batch_owned_100", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let arena = Arena::new();
                let msg = PersonOwned::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_batch_arena(c: &mut Criterion) {
    let data = create_test_data();

    c.bench_function("decode_batch_arena_100", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let arena = Arena::new();
                let msg = PersonArena::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_encode_owned(c: &mut Criterion) {
    let data = create_test_data();
    let arena = Arena::new();
    let msg = PersonOwned::decode(&data[..], &arena).unwrap();

    c.bench_function("encode_owned", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            black_box(&msg).encode(&mut buf).unwrap();
            black_box(buf);
        });
    });
}

fn bench_encode_arena(c: &mut Criterion) {
    let data = create_test_data();
    let arena = Arena::new();
    let msg = PersonArena::decode(&data[..], &arena).unwrap();

    c.bench_function("encode_arena", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            black_box(&msg).encode(&mut buf).unwrap();
            black_box(buf);
        });
    });
}

criterion_group!(
    benches,
    bench_decode_owned,
    bench_decode_arena,
    bench_decode_batch_owned,
    bench_decode_batch_arena,
    bench_encode_owned,
    bench_encode_arena,
);
criterion_main!(benches);
