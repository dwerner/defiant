use criterion::{criterion_group, criterion_main, Criterion};
use prost::{Arena, Message};
use std::error::Error;

pub mod benchmarks {
    include!(concat!(env!("OUT_DIR"), "/benchmarks.rs"));

    pub mod dataset {
        pub fn google_message1_proto2() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message1/proto2/dataset.google_message1_proto2.pb")
        }

        pub fn google_message1_proto3() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message1/proto3/dataset.google_message1_proto3.pb")
        }

        pub fn google_message2() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message2/dataset.google_message2.pb")
        }
    }

    pub mod proto2 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto2.rs"));
    }
    pub mod proto3 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto3.rs"));
    }
}

use crate::benchmarks::BenchmarkDataset;

fn load_dataset<'arena>(dataset: &[u8], arena: &'arena Arena) -> Result<BenchmarkDataset<'arena>, Box<dyn Error>> {
    Ok(BenchmarkDataset::decode(dataset, arena)?)
}

macro_rules! dataset {
    ($name: ident, $ty: ty) => {
        fn $name(criterion: &mut Criterion) {
            let dataset_bytes = crate::benchmarks::dataset::$name();
            let mut group = criterion.benchmark_group(&format!("dataset/{}", stringify!($name)));

            group.bench_function("decode", move |b| {
                let load_arena = Arena::new();
                let dataset = load_dataset(dataset_bytes, &load_arena).unwrap();
                b.iter(|| {
                    for buf in dataset.payload {
                        let arena = Arena::new();
                        let message = <$ty>::decode(*buf, &arena).unwrap();
                        std::hint::black_box(&message);
                    }
                });
            });

            group.bench_function("encode", move |b| {
                // Create arena and decode all messages once
                let arena = Arena::new();
                let load_arena = Arena::new();
                let dataset = load_dataset(dataset_bytes, &load_arena).unwrap();
                let messages: Vec<_> = dataset
                    .payload
                    .iter()
                    .map(|buf| <$ty>::decode(*buf, &arena).unwrap())
                    .collect();
                let mut buf = Vec::with_capacity(messages.iter().map(|m| m.encoded_len()).sum::<usize>());
                b.iter(|| {
                    buf.clear();
                    for message in &messages {
                        message.encode(&mut buf).unwrap();
                    }
                    std::hint::black_box(&buf);
                });
            });

            group.bench_function("encoded_len", move |b| {
                // Create arena and decode all messages once
                let arena = Arena::new();
                let load_arena = Arena::new();
                let dataset = load_dataset(dataset_bytes, &load_arena).unwrap();
                let messages: Vec<_> = dataset
                    .payload
                    .iter()
                    .map(|buf| <$ty>::decode(*buf, &arena).unwrap())
                    .collect();
                b.iter(|| {
                    let encoded_len = messages.iter().map(|m| m.encoded_len()).sum::<usize>();
                    std::hint::black_box(encoded_len)
                });
            });
        }
    };
}

dataset!(
    google_message1_proto2,
    crate::benchmarks::proto2::GoogleMessage1
);
dataset!(
    google_message1_proto3,
    crate::benchmarks::proto3::GoogleMessage1
);
dataset!(google_message2, crate::benchmarks::proto2::GoogleMessage2);

criterion_group!(
    dataset,
    google_message1_proto2,
    google_message1_proto3,
    google_message2,
);

criterion_main!(dataset);
