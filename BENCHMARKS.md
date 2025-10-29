# Defiant Benchmarks

## Machine Specifications

All benchmarks were run on the following hardware:

- **CPU**: AMD Ryzen 9 3900XT 12-Core Processor (24 threads)
- **Memory**: 64 GB DDR4
- **OS**: Manjaro Linux (Kernel 6.12.48)
- **Rust**: rustc 1.89.0

## Performance Benchmarks

### Speed Comparison (Google Protobuf Conformance Test Messages)

| Message | Library | Decode | Encode | Encoded Length |
|---------|---------|--------|--------|----------------|
| **google_message1_proto2** (228 bytes) | defiant | 256.41 ns | 161.24 ns | 35.99 ns |
| | prost | 289.94 ns | 166.73 ns | 36.09 ns |
| | pilota | 264.52 ns | 377.20 ns | 38.47 ns |
| **google_message1_proto3** (228 bytes) | defiant | 240.62 ns | 139.08 ns | 30.56 ns |
| | prost | 225.95 ns | 149.80 ns | 30.03 ns |
| | pilota | 247.09 ns | 1.03 µs† | 59.91 ns |
| **google_message2** (84,570 bytes) | defiant | 107.12 µs | 73.76 µs | 12.30 µs |
| | prost | 142.70 µs | 71.15 µs | 11.39 µs |
| | pilota | N/A‡ | N/A‡ | N/A‡ |

**Key Insights**:
- **Decode**: All three libraries perform similarly on small messages (~240-290ns). defiant is **25% faster** than prost on large messages (107µs vs 143µs)
- **Encode**: defiant is fastest for all messages. pilota is significantly slower on proto3 encoding (1µs vs 140-150ns)
- **Overall**: defiant offers the best balance of speed across all operations

† pilota proto3 encode is ~7x slower, likely due to FastStr/LinkedBytes overhead
‡ pilota doesn't support protobuf2 groups (deprecated feature)

### Memory Allocations Comparison

| Message | Library | Bytes Decoded | Allocations | Total Allocated | Overhead |
|---------|---------|--------------|-------------|-----------------|----------|
| **google_message1_proto2** | defiant | 228 bytes | **1 block** | 1,008 bytes | 4.4x |
| | prost | 228 bytes | 4 blocks | 174 bytes | 0.8x |
| | pilota | 228 bytes | **0 blocks** | 0 bytes | 0x |
| **google_message1_proto3** | defiant | 228 bytes | **1 block** | 1,008 bytes | 4.4x |
| | prost | 228 bytes | 4 blocks | 174 bytes | 0.8x |
| | pilota | 228 bytes | **0 blocks** | 0 bytes | 0x |
| **google_message2** | defiant | 84,570 bytes | **2 blocks** | 516,064 bytes | 6.1x |
| | prost | 84,570 bytes | 1,038 blocks | 725,345 bytes | 8.6x |
| | pilota | N/A | N/A | N/A | N/A† |

**Key Insights**:
- **defiant**: 1-2 allocations per message via arena. Higher overhead on small messages, but arena can be reused for zero allocations after warmup
- **prost**: Many small allocations per field. Lowest overhead on small messages, but scales poorly with message complexity (1,038 allocations for large message)
- **pilota**: True zero-copy using Arc-based string sharing. No allocations for decoding, but Arc overhead for cross-thread usage

† pilota doesn't support protobuf2 groups (deprecated feature)

## Comparison to Other Libraries

### Defiant vs prost vs pilota

| Aspect | **prost (owned)** | **defiant (arena)** | **pilota (Arc)** |
|--------|------------------|---------------------|------------------|
| **String type** | `String` | `&'arena str` | `FastStr` (Arc) |
| **Copy strategy** | Vec allocation | Single-copy to arena | Zero-copy (Arc split) |
| **Lifetime** | 'static (owned) | Tied to arena | 'static (Arc-owned) |
| **Send/Sync** | ✅ Yes | ❌ No | ✅ Yes |
| **Cross-thread** | ✅ Yes | ❌ Thread-local | ✅ Yes |
| **Memory reuse** | ❌ New alloc each | ✅ `arena.reset()` | ❌ New Arc each |
| **Allocations (reuse)** | Many per message | 0 after warmup | 1 Arc per decode |
| **Tokio compat** | ✅ Yes | ❌ Thread-per-core only | ✅ Yes |
| **Best for** | General purpose | High-throughput servers | Work-stealing runtimes |

### When to Use Each

**Defiant (Arena)**:
- ✅ Thread-per-core runtimes (monoio, glommio)
- ✅ Batch processing with arena reuse
- ✅ Large messages (>1KB)
- ✅ High-throughput request/response servers
- ❌ NOT for messages that need `Send` across threads
- ❌ NOT for Tokio work-stealing

**prost (Owned)**:
- ✅ General-purpose applications
- ✅ Messages passed between threads
- ✅ Tokio ecosystem
- ✅ Simplest mental model
- ❌ NOT ideal for high-throughput with many large messages

**pilota (Arc-based)**:
- ✅ True zero-copy (no data movement)
- ✅ Tokio work-stealing scheduler
- ✅ Messages passed between threads frequently
- ✅ Send+Sync requirement
- ❌ NOT for when Arc overhead matters
- ❌ NOT for memory reuse scenarios

## Testing Methodology

### Speed Benchmarks

Speed benchmarks use [Criterion.rs](https://github.com/bheisler/criterion.rs) with:
- **Warm-up period**: 3 seconds to stabilize CPU frequency and caches
- **Sample collection**: 100 samples with automatic iteration count determination
- **Statistical analysis**: Reports median time with confidence intervals

**Test procedure**:
1. Load the benchmark dataset (contains serialized protobuf messages)
2. For decode: Iterate through all messages in the dataset, decode each one
3. For encode: Decode all messages once (setup), then repeatedly encode them
4. For encoded_len: Decode all messages once (setup), then repeatedly calculate length

Benchmarks are run in release mode with full optimizations (LTO enabled, codegen-units=1).

**Running speed benchmarks**:
```bash
cargo bench -p benchmarks --bench dataset
```

### Memory Allocation Benchmarks

Memory allocation benchmarks use [dhat](https://github.com/nnethercote/dhat-rs), a heap profiler that tracks:
- **Total allocations**: Number of separate malloc/free calls
- **Total bytes allocated**: Sum of all allocation sizes
- **Peak memory**: Maximum heap usage during execution

**Test procedure**:
1. Load the benchmark dataset
2. Start dhat profiler
3. Decode all messages in the dataset sequentially
4. Report allocation statistics

This measures **per-message overhead** - the allocations needed to decode a single message. It does NOT include:
- The dataset loading itself (happens before profiler starts)
- Static/stack allocations
- Buffer allocations for the encoded bytes

**Why this matters**: Allocations are expensive in high-throughput scenarios:
- Each allocation requires syscalls and memory management overhead
- Many small allocations fragment the heap
- Arena/zero-copy strategies can dramatically reduce allocation count

**Running allocation benchmarks**:
```bash
cargo test --release -p benchmarks --bench allocations
```

Results are saved to `dhat-heap.json` and can be viewed with dhat's viewer.

### Dataset

All benchmarks use Google's official protobuf benchmark datasets:
- **google_message1_proto2**: Small message (228 bytes) with proto2 syntax
- **google_message1_proto3**: Small message (228 bytes) with proto3 syntax
- **google_message2**: Large, complex message (84,570 bytes) with nested structures and repeated fields

These are real-world message structures used in Google services, providing realistic performance data.
