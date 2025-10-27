use defiant::{Encode, Message};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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

fn main() {
    // Profile google_message1_proto2 (simple message)
    {
        let arena = defiant::Arena::new();
        let dataset_bytes = benchmarks::dataset::google_message1_proto2();
        let dataset = BenchmarkDataset::from_buf(dataset_bytes, &arena).unwrap();

        println!("\n=== Allocation Profile: google_message1_proto2 ===");
        println!("Number of messages in dataset: {}", dataset.payload.len());

        let _profiler = dhat::Profiler::new_heap();

        let mut total_bytes = 0;
        for buf in dataset.payload {
            let message = benchmarks::proto2::GoogleMessage1::from_buf(*buf, &arena).unwrap();
            total_bytes += buf.len();
            drop(message);
        }

        println!("Total bytes decoded: {}", total_bytes);
        println!("Stats saved to dhat-heap.json");
    }

    // Profile google_message1_proto3 (simple message, proto3)
    {
        let arena = defiant::Arena::new();
        let dataset_bytes = benchmarks::dataset::google_message1_proto3();
        let dataset = BenchmarkDataset::from_buf(dataset_bytes, &arena).unwrap();

        println!("\n=== Allocation Profile: google_message1_proto3 ===");
        println!("Number of messages in dataset: {}", dataset.payload.len());

        let _profiler = dhat::Profiler::new_heap();

        let mut total_bytes = 0;
        for buf in dataset.payload {
            let message = benchmarks::proto3::GoogleMessage1::from_buf(*buf, &arena).unwrap();
            total_bytes += buf.len();
            drop(message);
        }

        println!("Total bytes decoded: {}", total_bytes);
        println!("Stats saved to dhat-heap.json");
    }

    // Profile google_message2 (complex nested message)
    {
        let arena = defiant::Arena::new();
        let dataset_bytes = benchmarks::dataset::google_message2();
        let dataset = BenchmarkDataset::from_buf(dataset_bytes, &arena).unwrap();

        println!("\n=== Allocation Profile: google_message2 ===");
        println!("Number of messages in dataset: {}", dataset.payload.len());

        let _profiler = dhat::Profiler::new_heap();

        let mut total_bytes = 0;
        let mut successful = 0;
        let mut failed = 0;
        for buf in dataset.payload {
            match benchmarks::proto2::GoogleMessage2::from_buf(*buf, &arena) {
                Ok(message) => {
                    total_bytes += buf.len();
                    successful += 1;
                    drop(message);
                }
                Err(e) => {
                    eprintln!("Failed to decode message: {}", e);
                    failed += 1;
                }
            }
        }

        println!("Total bytes decoded: {}", total_bytes);
        println!("Successful: {}, Failed: {}", successful, failed);
        println!("Stats saved to dhat-heap.json");
    }
}
