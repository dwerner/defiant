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
        match tag {
            1 => prost::encoding::string::merge(wire_type, &mut self.name, buf, ctx),
            2 => prost::encoding::string::merge(wire_type, &mut self.email, buf, ctx),
            3 => prost::encoding::string::merge(wire_type, &mut self.phone, buf, ctx),
            4 => prost::encoding::string::merge(wire_type, &mut self.address, buf, ctx),
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

    fn clear(&mut self) {
        self.name.clear();
        self.email.clear();
        self.phone.clear();
        self.address.clear();
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
            prost::encoding::string::encode_ref(1, self.name, buf);
        }
        if !self.email.is_empty() {
            prost::encoding::string::encode_ref(2, self.email, buf);
        }
        if !self.phone.is_empty() {
            prost::encoding::string::encode_ref(3, self.phone, buf);
        }
        if !self.address.is_empty() {
            prost::encoding::string::encode_ref(4, self.address, buf);
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
                prost::encoding::string::encoded_len_ref(1, self.name)
            } else {
                0
            }
            + if !self.email.is_empty() {
                prost::encoding::string::encoded_len_ref(2, self.email)
            } else {
                0
            }
            + if !self.phone.is_empty() {
                prost::encoding::string::encoded_len_ref(3, self.phone)
            } else {
                0
            }
            + if !self.address.is_empty() {
                prost::encoding::string::encoded_len_ref(4, self.address)
            } else {
                0
            }
    }

    fn clear(&mut self) {
        self.name = "";
        self.email = "";
        self.phone = "";
        self.address = "";
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
