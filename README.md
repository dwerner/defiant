# Defiant

`defiant` is an arena-based [Protocol Buffers](https://protobuf.dev/) implementation for Rust, optimized for thread-per-core architectures. It generates efficient, memory-safe code with minimal allocations by using arena allocation and a View/Builder pattern.

## Quick Start

```rust
use defiant::{Arena, Message, Encode};

#[derive(Message)]
struct Person<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(int32, tag = 2)]
    age: i32,
}

let arena = Arena::new();

// Decode from wire format
let person = PersonBuilder::decode(&bytes, &arena)?.freeze();
assert_eq!(person.name, "Alice");

// Encode to wire format
let encoded = person.encode_to_vec();
```

## Core Concepts

### Arena Allocation

All strings, bytes, repeated fields, and nested messages are allocated in a single arena:

```rust
let arena = Arena::new();

// Decode multiple messages into the same arena
let msg1 = Message1Builder::decode(&bytes1, &arena)?.freeze();
let msg2 = Message2Builder::decode(&bytes2, &arena)?.freeze();

// All data shares the same arena - only 1-2 total allocations!
```

**Benefits:**
- **1-2 allocations** per message (vs 100+ for traditional approach)
- **Cache-friendly** memory layout
- **Batch processing**: Reset arena and reuse for next batch
- **Zero allocations** after warmup when reusing arenas

**Tradeoff:**
- Messages are **not `Send`** - tied to the thread that owns the arena
- Requires thread-per-core runtime (e.g., monoio, glommio)

### View/Builder Pattern

Defiant uses separate types for reading and writing:

```rust
// View: Immutable, zero-cost, holds references
struct PersonView<'arena> {
    name: &'arena str,  // Points into arena
    age: i32,
}

// Builder: Mutable, accumulates during decode
struct PersonBuilder<'arena> {
    name: &'arena str,  // Allocated in arena during decode
    age: i32,
}

// Convert Builder → View (zero-cost)
let view = builder.freeze();
```

**Why two types?**
- Views are **immutable and cheap to pass around**
- Builders **accumulate fields during decoding**
- Clear separation between construction and usage
- Enables zero-cost `freeze()` conversion

## When to Use Defiant

### ✅ Ideal Use Cases

- **Thread-per-core architectures** (monoio, glommio)
- **High-throughput request/response servers** with thread pinning
- **Batch processing** (decode many, reset arena, repeat)
- **Large messages** (>1KB) where allocation cost matters
- **Minimum allocations** - critical for low-latency systems

### ❌ Not Suitable For

- **Tokio work-stealing scheduler** (messages aren't `Send`)
- **Messages passed between threads**
- **Long-lived messages** that outlive arena lifetime
- **Traditional async multi-threading**

### Alternative: Use `prost` (owned) or `pilota` (Arc-based) if you need:
- `Send + Sync` messages
- Work-stealing schedulers
- Messages that move between threads
- Simpler mental model

See [BENCHMARKS.md](BENCHMARKS.md) for detailed performance comparisons.

## API Examples

### Basic Message

```protobuf
message Person {
  string name = 1;
  int32 age = 2;
}
```

```rust
#[derive(Message)]
struct Person<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(int32, tag = 2)]
    age: i32,
}

// Encoding
let person = Person { name: "Alice", age: 30 };
let bytes = person.encode_to_vec();

// Decoding
let arena = Arena::new();
let decoded = PersonBuilder::decode(&bytes[..], &arena)?.freeze();
```

### Repeated Fields

```protobuf
message Company {
  string name = 1;
  repeated string departments = 2;
}
```

```rust
#[derive(Message)]
struct Company<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(string, repeated, tag = 2)]
    departments: &'arena [&'arena str],  // Slice of arena strings
}

let company = Company {
    name: "Acme",
    departments: &["Engineering", "Sales", "Marketing"],
};
```

### Nested Messages

```protobuf
message Company {
  string name = 1;
  Person ceo = 2;
  repeated Person employees = 3;
}
```

```rust
#[derive(Message)]
struct Company<'arena> {
    #[defiant(string, tag = 1)]
    name: &'arena str,
    #[defiant(message, tag = 2)]
    ceo: Option<&'arena Person<'arena>>,  // Optional reference
    #[defiant(message, repeated, tag = 3)]
    employees: &'arena [&'arena Person<'arena>],  // Slice of references
}
```

### Oneofs

```protobuf
message Notification {
  oneof payload {
    string text = 1;
    Image image = 2;
    int32 count = 3;
  }
}
```

```rust
#[derive(Message)]
struct Notification<'arena> {
    #[defiant(oneof = "Payload", tags = "1, 2, 3")]
    payload: Option<Payload<'arena>>,
}

#[derive(Oneof)]
enum Payload<'arena> {
    #[defiant(string, tag = 1)]
    Text(&'arena str),
    #[defiant(message, tag = 2)]
    Image(Image<'arena>),
    #[defiant(int32, tag = 3)]
    Count(i32),
}
```

### Maps

```protobuf
message Config {
  map<string, string> settings = 1;
  map<int32, string> flags = 2;
}
```

```rust
use defiant::ArenaMap;

#[derive(Message)]
struct Config<'arena> {
    #[defiant(arena_map = "string, string", tag = 1)]
    settings: ArenaMap<'arena, &'arena str, &'arena str>,
    #[defiant(arena_map = "int32, string", tag = 2)]
    flags: ArenaMap<'arena, i32, &'arena str>,
}

// Create map
let settings = ArenaMap::new(&[
    ("host", "localhost"),
    ("port", "8080"),
]);

let config = Config { settings, flags: ArenaMap::new(&[]) };
```

## Type Reference

| Protobuf Type | Rust Type (View) |
|---------------|------------------|
| `double` | `f64` |
| `float` | `f32` |
| `int32`, `sint32`, `sfixed32` | `i32` |
| `int64`, `sint64`, `sfixed64` | `i64` |
| `uint32`, `fixed32` | `u32` |
| `uint64`, `fixed64` | `u64` |
| `bool` | `bool` |
| `string` | `&'arena str` |
| `bytes` | `&'arena [u8]` |
| `message` | `&'arena MessageType<'arena>` |
| `repeated T` | `&'arena [T]` |
| `map<K,V>` | `ArenaMap<'arena, K, V>` |
| `oneof` | `Option<EnumType<'arena>>` |

## Setup

Add to `Cargo.toml`:

```toml
[dependencies]
defiant = "0.1"
defiant-types = "0.1"  # For well-known types (Timestamp, Duration, etc.)

[build-dependencies]
defiant-build = "0.1"
```

Create `build.rs`:

```rust
fn main() {
    let arena = defiant::Arena::new();

    defiant_build::Config::new(&arena)
        .compile_protos(&["src/messages.proto"], &["src/"])
        .unwrap();
}
```

Include generated code:

```rust
// In your lib.rs or main.rs
include!(concat!(env!("OUT_DIR"), "/messages.rs"));
```

## Configuration

```rust
defiant_build::Config::new(&arena)
    // Custom re-export paths
    .defiant_path("::my_crate::defiant")
    .defiant_types_path("::my_crate::types")

    // Use BTreeMap instead of HashMap for maps
    .btree_map(["."])

    // Add custom derives to generated types
    .type_attribute(".", "#[derive(serde::Serialize)]")

    // Add custom derives to specific messages
    .type_attribute("Person", "#[derive(Eq)]")

    .compile_protos(&["src/messages.proto"], &["src/"])
    .unwrap();
```

## Memory Safety

The borrow checker ensures arenas outlive all decoded messages:

```rust
// ❌ Compile error: arena dropped too early
let person = {
    let arena = Arena::new();
    PersonBuilder::decode(&bytes, &arena)?.freeze()
}; // arena dropped here, person contains dangling references!

// ✅ Correct: arena outlives all messages
let arena = Arena::new();
let person1 = PersonBuilder::decode(&bytes1, &arena)?.freeze();
let person2 = PersonBuilder::decode(&bytes2, &arena)?.freeze();
// Both share the same arena
```

Arena reuse for batch processing:

```rust
let mut arena = Arena::new();

loop {
    let request_bytes = read_request()?;

    // Decode into arena
    let request = RequestBuilder::decode(&request_bytes, &arena)?.freeze();

    // Process and respond
    let response = handle_request(&request);
    let response_bytes = response.encode_to_vec();
    send_response(&response_bytes)?;

    // Reset arena for next request (zero allocations!)
    arena.reset();
}
```

## Performance

See [BENCHMARKS.md](BENCHMARKS.md) for detailed benchmarks and comparisons.

**Key results:**
- **1-2 total allocations** per message (vs 100+ traditional)
- **252ns decode** for small messages (228 bytes)
- **105µs decode** for large messages (84KB)
- **Arena reuse: 0 allocations** after warmup

## Architecture Patterns

### Thread-Per-Core Server (monoio)

```rust
use monoio::net::TcpListener;

#[monoio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        // Each connection on same thread - arena stays thread-local
        monoio::spawn(async move {
            let mut arena = Arena::new();

            loop {
                let bytes = read_from_stream(&stream).await?;

                // Decode request
                let request = RequestBuilder::decode(&bytes, &arena)?.freeze();

                // Process
                let response = handle(&request);

                // Send response
                write_to_stream(&stream, &response.encode_to_vec()).await?;

                // Reset for next request - zero allocations!
                arena.reset();
            }
        });
    }
}
```

### Batch Processing

```rust
let mut arena = Arena::new();
let mut messages = Vec::new();

// Decode batch
for bytes in batch_of_wire_data {
    let msg = MessageBuilder::decode(&bytes, &arena)?.freeze();
    messages.push(msg);
}

// Process all messages (all in same arena)
process_batch(&messages);

// Reset arena, ready for next batch
arena.reset();
messages.clear();
```

## No-std Support

Defiant supports `no_std` with `alloc`:

```toml
[dependencies]
defiant = { version = "0.1", default-features = false }
```

## Differences from Prost

| Feature | Prost | Defiant |
|---------|-------|---------|
| String type | `String` | `&'arena str` |
| Repeated fields | `Vec<T>` | `&'arena [T]` |
| Message fields | `Option<Box<M>>` | `Option<&'arena M<'arena>>` |
| Allocations | Many per message | 1-2 per message |
| Send/Sync | ✅ Yes | ❌ No (arena-bound) |
| Runtime | Any | Thread-per-core |
| API complexity | Simple | Medium (View/Builder) |

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

`defiant` is a fork of [prost](https://github.com/tokio-rs/prost) and is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.

Original Copyright 2022 Dan Burkert & Tokio Contributors
