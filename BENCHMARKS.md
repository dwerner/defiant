# Defiant Benchmarks

## Machine Specifications

All benchmarks were run on the following hardware:

- **CPU**: AMD Ryzen 9 3900XT 12-Core Processor (24 threads)
- **Memory**: 64 GB DDR4
- **OS**: Manjaro Linux (Kernel 6.12.48)
- **Rust**: rustc 1.89.0

## Benchmark Results

### Speed Benchmarks

Google protobuf conformance test messages:

| Message | Size | Operation | Time |
|---------|------|-----------|------|
| **google_message1_proto2** | 228 bytes | Decode | 252.62 ns |
| | | Encode | 155.28 ns |
| | | Encoded length | 35.22 ns |
| **google_message1_proto3** | 228 bytes | Decode | 236.93 ns |
| | | Encode | 137.12 ns |
| | | Encoded length | 30.13 ns |
| **google_message2** | 84,570 bytes | Decode | 105.63 µs |
| | | Encode | 72.33 µs |
| | | Encoded length | 11.81 µs |

### Memory Allocation Benchmarks

**Key Result**: Arena allocation provides extremely efficient memory usage with minimal allocations.

| Message | Bytes Decoded | Allocations | Total Allocated | Efficiency |
|---------|--------------|-------------|-----------------|------------|
| google_message1_proto2 | 228 bytes | **1 block** | 1,008 bytes | 4.4x overhead |
| google_message1_proto3 | 228 bytes | **1 block** | 1,008 bytes | 4.4x overhead |
| google_message2 | 84,570 bytes | **2 blocks** | 516,064 bytes | 6.1x overhead |

**What this means**:
- Only 1-2 allocations per message, regardless of complexity
- All strings, bytes, repeated fields, and nested messages allocated in a single arena
- No per-field allocations (traditional approach would have 100+ allocations for complex messages)
- Arena can be reused across multiple messages for batch processing

## Comparison: Defiant vs Other Strategies

### Three Approaches to Zero-Copy Protobuf

| Aspect | **Owned (prost)** | **Defiant Arena** | **Pilota (Arc-based)** |
|--------|------------------|-------------------|------------------------|
| **String type** | `String` | `&'arena str` | `FastStr` (Arc<[u8]>) |
| **Copy strategy** | Vec allocation | Single-copy to arena | Zero-copy (Arc split) |
| **Lifetime** | 'static (owned) | Tied to arena | 'static (Arc-owned) |
| **Send/Sync** | ✅ Send+Sync | ❌ NOT Send | ✅ Send+Sync |
| **Cross-thread** | ✅ Can move | ❌ Tied to thread | ✅ Can move freely |
| **Memory reuse** | ❌ New alloc each | ✅ arena.reset() | ❌ New Arc each |
| **Allocations (reuse)** | Many per message | 0 after warmup | 1 Arc per decode |
| **Tokio compat** | ✅ Works | ❌ Requires monoio | ✅ Works |
| **Best for** | General purpose | Thread-per-core | Work-stealing |

### Performance Comparison (Small Messages ~228 bytes)

Based on previous benchmarks from `/home/dan/Development/en/memory/PILOTA_VS_PROST_ARENA.md`:

| Implementation | Decode Time | Speedup vs Owned |
|----------------|-------------|------------------|
| prost (owned) | ~56.6 ns | baseline |
| prost arena (old) | ~55.2 ns | 2.5% faster |
| **defiant arena (current)** | **236.93 ns** | **4.2x slower** |

**Note**: Current defiant results are slower than previous prost arena benchmarks. This warrants investigation - possible causes:
- Different message structure in conformance tests vs custom benchmarks
- Additional safety checks in generated code
- Debug assertions enabled
- Different benchmark methodology

### Performance Scaling (Message Size)

Previous prost arena benchmarks showed arena advantage increases with message size:

| Message Size | Owned | Arena | Speedup |
|--------------|-------|-------|---------|
| 12 bytes | 56.6 ns | 55.2 ns | 2.5% |
| 1,002 bytes | 658 ns | 410 ns | 37.6% |
| 10,003 bytes | 5.88 µs | 2.82 µs | 52.0% |
| 100,000 bytes | 60.9 µs | 20.8 µs | 65.8% |

**Pattern**: Arena wins by larger margins as message size increases.

## Architecture Tradeoffs

### When to Use Defiant Arena

✅ **Ideal for:**
- Thread-per-core runtimes (monoio, glommio)
- Batch processing (decode many messages, reset arena)
- Large messages (>1KB) where allocation overhead matters
- Control over message lifetime
- Minimum allocations after warmup
- High-throughput request/response servers with thread-pinning

❌ **Not suitable for:**
- Messages that need to be `Send` across threads
- Tokio work-stealing scheduler
- Long-lived messages that outlive the arena
- Applications requiring traditional async multi-threading

### When to Use Traditional Owned Approach

✅ **Ideal for:**
- General-purpose applications
- Messages passed between threads
- Long-lived messages
- Tokio ecosystem compatibility
- Simplest mental model

❌ **Not ideal for:**
- High-throughput scenarios with many allocations
- Large messages where allocation cost dominates

### When to Use Pilota (Arc-based)

✅ **Ideal for:**
- True zero-copy (no data movement at all)
- Tokio work-stealing scheduler
- Messages passed between threads frequently
- Buffer sharing across messages
- Send+Sync requirement

❌ **Not suitable for:**
- When Arc overhead is unacceptable
- Memory reuse scenarios
- Thread-per-core architectures

## Running Benchmarks

### Speed Benchmarks

```bash
cargo bench --package benchmarks --bench dataset
```

### Memory Allocation Benchmarks

```bash
cargo bench --package benchmarks --bench allocations
```

Results are saved to `dhat-heap.json` and can be viewed with dhat's viewer.

### Custom Message Benchmarks

```bash
cd benchmarks
cargo bench -- --sample-size 100
```

## API Design Philosophy

Defiant uses a **View/Builder pattern** to maximize performance while maintaining safety:

### Views (Encoding)

```rust
// View: Immutable, zero-cost, holds references
#[derive(Message)]
struct Person<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(int32, tag = 2)]
    age: i32,
}

let person = Person { name: "Alice", age: 30 };
let bytes = person.encode_to_vec();  // Fast encoding
```

**Characteristics**:
- Zero-cost: Just references to arena-allocated data
- Immutable: Safe to share within arena lifetime
- Fast encoding: No allocations, direct wire format writing

### Builders (Decoding)

```rust
let arena = Arena::new();
let person = PersonBuilder::decode(&bytes, &arena)?
    .freeze();  // Convert to View

assert_eq!(person.name, "Alice");
```

**Characteristics**:
- Mutable: Accumulates fields during decode
- Arena-aware: Allocates strings/bytes into arena
- `.freeze()`: Zero-cost conversion to immutable View

## Memory Safety

Defiant maintains Rust's safety guarantees:

```rust
let person = {
    let arena = Arena::new();
    PersonBuilder::decode(&bytes, &arena)?.freeze()
}; // ❌ Compile error: arena dropped, person contains dangling references

// ✅ Correct: Arena must outlive all decoded messages
let arena = Arena::new();
let person1 = PersonBuilder::decode(&bytes1, &arena)?.freeze();
let person2 = PersonBuilder::decode(&bytes2, &arena)?.freeze();
// Both person1 and person2 share the same arena
```

The borrow checker ensures messages cannot outlive their arena.

## Future Work

- [ ] Investigate performance regression vs previous prost arena benchmarks
- [ ] Add comparative benchmarks vs protobuf-rs, prost (owned), and pilota
- [ ] Benchmark arena reuse in request/response scenarios
- [ ] Profile with different arena initial sizes
- [ ] Add benchmarks for monoio integration
- [ ] Measure arena fragmentation under various workloads
