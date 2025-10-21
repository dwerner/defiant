use prost::Message;

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

fn load_dataset(dataset: &[u8]) -> BenchmarkDataset {
    BenchmarkDataset::decode(dataset).unwrap()
}

fn profile_allocations<M>(name: &str, dataset: &'static [u8])
where
    M: prost::Message + Default,
{
    let dataset = load_dataset(dataset);

    println!("\n=== Allocation Profile: {} ===", name);
    println!("Number of messages in dataset: {}", dataset.payload.len());

    let _profiler = dhat::Profiler::new_heap();

    // Decode all messages to count allocations
    let mut total_bytes = 0;
    for buf in &dataset.payload {
        let message = M::decode(buf.as_slice()).unwrap();
        total_bytes += buf.len();
        drop(message);
    }

    println!("Total bytes decoded: {}", total_bytes);
    println!("Stats saved to dhat-heap.json");
    println!("View with: https://nnethercote.github.io/dh_view/dh_view.html");
}

fn main() {
    // Profile google_message1_proto2 (simple message)
    profile_allocations::<benchmarks::proto2::GoogleMessage1>(
        "google_message1_proto2",
        benchmarks::dataset::google_message1_proto2(),
    );

    // Profile google_message1_proto3 (simple message, proto3)
    profile_allocations::<benchmarks::proto3::GoogleMessage1>(
        "google_message1_proto3",
        benchmarks::dataset::google_message1_proto3(),
    );

    // Profile google_message2 (complex nested message - this will show the most allocations)
    profile_allocations::<benchmarks::proto2::GoogleMessage2>(
        "google_message2",
        benchmarks::dataset::google_message2(),
    );
}
