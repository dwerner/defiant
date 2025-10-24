use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use prost::{Arena, Message};
use rand::Rng;

// Traditional owned message
#[derive(Debug, Default)]
struct MessageOwned {
    data: Vec<String>,
}

impl<'arena> Message<'arena> for MessageOwned {
    fn new_in(_arena: &'arena Arena) -> Self {
        Self::default()
    }

    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        for (i, value) in self.data.iter().enumerate() {
            if !value.is_empty() {
                prost::encoding::string::encode((i + 1) as u32, value, buf);
            }
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

        if tag == 0 || tag > 1000 {
            return prost::encoding::skip_field(wire_type, tag, buf, ctx);
        }

        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)? as usize;
        let mut value = String::new();
        value.reserve(len);
        unsafe {
            value.as_mut_vec().resize(len, 0);
            buf.copy_to_slice(value.as_mut_vec());
        }
        if !std::str::from_utf8(value.as_bytes()).is_ok() {
            return Err(prost::DecodeError::new("invalid UTF-8"));
        }

        // Ensure vec is large enough
        let idx = (tag - 1) as usize;
        if self.data.len() <= idx {
            self.data.resize(idx + 1, String::new());
        }
        self.data[idx] = value;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if !v.is_empty() {
                    prost::encoding::string::encoded_len((i + 1) as u32, v)
                } else {
                    0
                }
            })
            .sum()
    }
}

// Arena-allocated message
#[derive(Debug)]
struct MessageArena<'arena> {
    data: Vec<&'arena str>,
}

impl<'arena> Default for MessageArena<'arena> {
    fn default() -> Self {
        MessageArena { data: Vec::new() }
    }
}

impl<'arena> Message<'arena> for MessageArena<'arena> {
    fn new_in(_arena: &'arena Arena) -> Self {
        Self::default()
    }

    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        for (i, value) in self.data.iter().enumerate() {
            if !value.is_empty() {
                prost::encoding::string::encode((i + 1) as u32, &value.to_string(), buf);
            }
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
        if tag == 0 || tag > 1000 {
            return prost::encoding::skip_field(wire_type, tag, buf, ctx);
        }

        let value = prost::encoding::string::merge_arena(wire_type, buf, arena, ctx)?;

        // Ensure vec is large enough
        let idx = (tag - 1) as usize;
        if self.data.len() <= idx {
            self.data.resize(idx + 1, "");
        }
        self.data[idx] = value;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if !v.is_empty() {
                    prost::encoding::string::encoded_len((i + 1) as u32, &v.to_string())
                } else {
                    0
                }
            })
            .sum()
    }
}

/// Creates a protobuf message of approximately the given size
fn create_message_data(target_size: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut current_size = 0;
    let mut field_num = 1u32;

    // Use increasingly larger strings to reach target size
    while current_size < target_size {
        let remaining = target_size - current_size;
        // Field size: make it roughly 20-100 bytes per field
        let field_size = remaining.min(80);

        if field_size < 10 {
            break; // Don't add tiny fields at the end
        }

        // Encode: tag + length + data
        prost::encoding::encode_key(field_num, prost::encoding::WireType::LengthDelimited, &mut data);
        prost::encoding::encode_varint(field_size as u64, &mut data);

        // Generate string data (repeated pattern for simplicity)
        let pattern = format!("field{:03}", field_num);
        let pattern_bytes = pattern.as_bytes();
        for i in 0..field_size {
            data.push(pattern_bytes[i % pattern_bytes.len()]);
        }

        current_size = data.len();
        field_num += 1;
    }

    data
}

/// Creates messages with random sizes within a range
fn create_random_size_messages(count: usize, min_size: usize, max_size: usize) -> Vec<Vec<u8>> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| {
            let size = rng.gen_range(min_size..=max_size);
            create_message_data(size)
        })
        .collect()
}

fn bench_decode_sizes_owned(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_owned_by_size");

    for size in [10, 50, 100, 500, 1_000, 5_000, 10_000, 50_000, 100_000] {
        let data = create_message_data(size);
        let actual_size = data.len();

        group.bench_with_input(
            BenchmarkId::from_parameter(actual_size),
            &data,
            |b, data| {
                b.iter(|| {
                    let arena = Arena::new();
                    let msg = MessageOwned::decode(black_box(&data[..]), &arena).unwrap();
                    black_box(msg);
                });
            },
        );
    }
    group.finish();
}

fn bench_decode_sizes_arena(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_arena_by_size");

    for size in [10, 50, 100, 500, 1_000, 5_000, 10_000, 50_000, 100_000] {
        let data = create_message_data(size);
        let actual_size = data.len();

        group.bench_with_input(
            BenchmarkId::from_parameter(actual_size),
            &data,
            |b, data| {
                // Reused arena - typical use case for batch processing
                let mut arena = Arena::with_capacity(size * 2);
                b.iter(|| {
                    arena.reset();
                    let msg = MessageArena::decode(black_box(&data[..]), &arena).unwrap();
                    black_box(msg);
                });
            },
        );
    }
    group.finish();
}

fn bench_decode_random_small_owned(c: &mut Criterion) {
    let messages = create_random_size_messages(100, 10, 100);

    c.bench_function("decode_owned_random_10_100", |b| {
        b.iter(|| {
            for data in &messages {
                let arena = Arena::new();
                let msg = MessageOwned::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_random_small_arena(c: &mut Criterion) {
    let messages = create_random_size_messages(100, 10, 100);

    c.bench_function("decode_arena_random_10_100", |b| {
        // Reused arena across all messages
        let mut arena = Arena::with_capacity(200);
        b.iter(|| {
            for data in &messages {
                arena.reset();
                let msg = MessageArena::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_random_medium_owned(c: &mut Criterion) {
    let messages = create_random_size_messages(100, 100, 1_000);

    c.bench_function("decode_owned_random_100_1k", |b| {
        b.iter(|| {
            for data in &messages {
                let arena = Arena::new();
                let msg = MessageOwned::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_random_medium_arena(c: &mut Criterion) {
    let messages = create_random_size_messages(100, 100, 1_000);

    c.bench_function("decode_arena_random_100_1k", |b| {
        // Reused arena across all messages
        let mut arena = Arena::with_capacity(1_000);
        b.iter(|| {
            for data in &messages {
                arena.reset();
                let msg = MessageArena::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_random_large_owned(c: &mut Criterion) {
    let messages = create_random_size_messages(50, 1_000, 10_000);

    c.bench_function("decode_owned_random_1k_10k", |b| {
        b.iter(|| {
            for data in &messages {
                let arena = Arena::new();
                let msg = MessageOwned::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

fn bench_decode_random_large_arena(c: &mut Criterion) {
    let messages = create_random_size_messages(50, 1_000, 10_000);

    c.bench_function("decode_arena_random_1k_10k", |b| {
        // Reused arena across all messages
        let mut arena = Arena::with_capacity(10_000);
        b.iter(|| {
            for data in &messages {
                arena.reset();
                let msg = MessageArena::decode(black_box(&data[..]), &arena).unwrap();
                black_box(msg);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_decode_sizes_owned,
    bench_decode_sizes_arena,
    bench_decode_random_small_owned,
    bench_decode_random_small_arena,
    bench_decode_random_medium_owned,
    bench_decode_random_medium_arena,
    bench_decode_random_large_owned,
    bench_decode_random_large_arena,
);
criterion_main!(benches);
