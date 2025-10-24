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

    while current_size < target_size {
        let remaining = target_size - current_size;
        let field_size = remaining.min(80);

        if field_size < 10 {
            break;
        }

        prost::encoding::encode_key(field_num, prost::encoding::WireType::LengthDelimited, &mut data);
        prost::encoding::encode_varint(field_size as u64, &mut data);

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

fn main() {
    println!("\n=== ARENA BYTES USED COMPARISON ===\n");

    for size in [100, 1_000, 10_000, 100_000] {
        let data = create_message_data(size);
        let actual_size = data.len();

        println!("--- Message size: {} bytes ---", actual_size);

        // Owned version - measure arena usage (shouldn't use arena for strings)
        let arena = Arena::new();
        let before_owned = arena.allocated_bytes();
        let msg_owned = MessageOwned::decode(&data[..], &arena).unwrap();
        let after_owned = arena.allocated_bytes();
        let owned_heap = std::mem::size_of_val(&msg_owned)
            + msg_owned.data.capacity() * std::mem::size_of::<String>()
            + msg_owned.data.iter().map(|s| s.capacity()).sum::<usize>();

        println!("Owned:");
        println!("  Arena bytes used: {} (should be 0)", after_owned - before_owned);
        println!("  Heap bytes (estimated): {}", owned_heap);
        drop(msg_owned);

        // Arena version
        let arena = Arena::new();
        let before_arena = arena.allocated_bytes();
        let msg_arena = MessageArena::decode(&data[..], &arena).unwrap();
        let after_arena = arena.allocated_bytes();
        let arena_heap = std::mem::size_of_val(&msg_arena)
            + msg_arena.data.capacity() * std::mem::size_of::<&str>();

        println!("Arena:");
        println!("  Arena bytes used: {}", after_arena - before_arena);
        println!("  Stack/heap for Vec: {}", arena_heap);
        println!("  Total: {}", (after_arena - before_arena) + arena_heap);
        println!();
    }

    println!("\n=== BATCH PROCESSING (100 iterations) ===\n");

    for size in [100, 1_000, 10_000, 100_000] {
        let data = create_message_data(size);
        let actual_size = data.len();

        println!("--- Message size: {} bytes ---", actual_size);

        // Owned - fresh arena each time
        let mut total_arena_bytes_owned = 0;
        for _ in 0..100 {
            let arena = Arena::new();
            let before = arena.allocated_bytes();
            let msg = MessageOwned::decode(&data[..], &arena).unwrap();
            let after = arena.allocated_bytes();
            total_arena_bytes_owned += after - before;
            drop(msg);
        }
        println!("Owned (100 iterations, fresh arena each): {} arena bytes total", total_arena_bytes_owned);

        // Arena - reused arena
        let mut arena = Arena::with_capacity(size * 2);
        let mut total_arena_bytes_reused = 0;
        let initial_capacity = arena.allocated_bytes();

        for _ in 0..100 {
            let before = arena.allocated_bytes();
            arena.reset();
            let msg = MessageArena::decode(&data[..], &arena).unwrap();
            let after = arena.allocated_bytes();
            total_arena_bytes_reused += after - before;
            drop(msg);
        }

        println!("Arena (100 iterations, reused):         {} arena bytes total (initial capacity: {})",
            total_arena_bytes_reused, initial_capacity);
        println!("  Savings: {} bytes ({:.1}x)",
            total_arena_bytes_owned.saturating_sub(total_arena_bytes_reused),
            if total_arena_bytes_reused > 0 { total_arena_bytes_owned as f64 / total_arena_bytes_reused as f64 } else { 0.0 });
        println!();
    }
}
