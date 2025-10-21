//! Comparison of heap allocations: Owned vs Arena

use prost::{Arena, Message};

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

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn test_owned_allocations() {
    let _profiler = dhat::Profiler::new_heap();

    let data = create_test_data();
    let arena = Arena::new();

    println!("\n=== OWNED APPROACH ===");
    let stats_start = dhat::HeapStats::get();

    // Decode 100 messages with owned strings
    for _ in 0..100 {
        let msg = PersonOwned::decode(&data[..], &arena).unwrap();
        std::hint::black_box(&msg);
    }

    let stats_end = dhat::HeapStats::get();

    let allocs = stats_end.total_blocks - stats_start.total_blocks;
    let bytes = stats_end.total_bytes - stats_start.total_bytes;

    println!("Total allocations: {}", allocs);
    println!("Total bytes allocated: {}", bytes);
    println!("Average allocations per message: {}", allocs / 100);
    println!("Average bytes per message: {}", bytes / 100);
}

#[test]
fn test_arena_allocations() {
    let _profiler = dhat::Profiler::new_heap();

    let data = create_test_data();

    println!("\n=== ARENA APPROACH ===");
    let stats_start = dhat::HeapStats::get();

    // Decode 100 messages with arena allocation
    for _ in 0..100 {
        let arena = Arena::new();
        let msg = PersonArena::decode(&data[..], &arena).unwrap();
        std::hint::black_box(&msg);
    }

    let stats_end = dhat::HeapStats::get();

    let allocs = stats_end.total_blocks - stats_start.total_blocks;
    let bytes = stats_end.total_bytes - stats_start.total_bytes;

    println!("Total allocations: {}", allocs);
    println!("Total bytes allocated: {}", bytes);
    println!("Average allocations per message: {}", allocs / 100);
    println!("Average bytes per message: {}", bytes / 100);
}

#[test]
fn test_allocation_comparison() {
    println!("\n=== ALLOCATION ANALYSIS ===\n");

    println!("Message structure:");
    println!("  - name: 13 bytes");
    println!("  - email: 25 bytes");
    println!("  - phone: 11 bytes");
    println!("  - address: 35 bytes");
    println!("  - Total string data: 84 bytes\n");

    println!("OWNED APPROACH:");
    println!("  - Each message requires 4 separate heap allocations (one per String)");
    println!("  - Each String has overhead (~24 bytes on 64-bit systems)");
    println!("  - Each allocation has allocator overhead");
    println!("  - Total per message: ~4 allocations + malloc overhead\n");

    println!("ARENA APPROACH:");
    println!("  - Arena allocates one contiguous chunk (default 4KB)");
    println!("  - All 4 strings allocated sequentially within arena");
    println!("  - Zero individual allocations for strings");
    println!("  - Total per message: ~1 allocation (arena itself)\n");

    println!("BENEFITS:");
    println!("  ✓ Fewer allocations = faster decoding");
    println!("  ✓ Better cache locality (contiguous memory)");
    println!("  ✓ Faster cleanup (single arena drop)");
    println!("  ✓ Reduced allocator contention");
    println!("  ✓ Predictable memory layout");
}
