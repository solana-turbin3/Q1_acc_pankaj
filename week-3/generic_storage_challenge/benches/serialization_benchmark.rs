use borsh::{BorshDeserialize, BorshSerialize};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use generic_storage_challenge::{
    borsh_serializer::BorshSerializer, json_serializer::JsonSerializer,
    wincode_serializer::WincodeSerializer, Storage,
};
use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

#[derive(
    Debug,
    PartialEq,
    Clone,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    SchemaWrite,
    SchemaRead,
)]
struct BenchPerson {
    name: String,
    age: u32,
    email: String,
    bio: String,
    data: Vec<u8>,
}

fn criterion_benchmark(c: &mut Criterion) {
    let person = BenchPerson {
        name: "andre".to_string(),
        age: 32,
        email: "andre@turbin3.com".to_string(),
        bio: "coolest instructor of the accelerated turbin3".to_string(),
        data: vec![0u8; 1024],
    };

    let mut group = c.benchmark_group("serialization");

    // Setting measurement time to 10 seconds for more accurate results
    group.measurement_time(std::time::Duration::from_secs(10));

    // Borsh
    group.bench_function("borsh_save", |b| {
        let mut storage = Storage::new(BorshSerializer);
        b.iter(|| {
            storage.save(black_box(&person)).unwrap();
        })
    });
    group.bench_function("borsh_load", |b| {
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person).unwrap();
        b.iter(|| {
            let _ = storage.load().unwrap();
        })
    });

    // Wincode
    group.bench_function("wincode_save", |b| {
        let mut storage = Storage::new(WincodeSerializer);
        b.iter(|| {
            storage.save(black_box(&person)).unwrap();
        })
    });
    group.bench_function("wincode_load", |b| {
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person).unwrap();
        b.iter(|| {
            let _ = storage.load().unwrap();
        })
    });

    // Json
    group.bench_function("json_save", |b| {
        let mut storage = Storage::new(JsonSerializer);
        b.iter(|| {
            storage.save(black_box(&person)).unwrap();
        })
    });
    group.bench_function("json_load", |b| {
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person).unwrap();
        b.iter(|| {
            let _ = storage.load().unwrap();
        })
    });

    // Conversion Benchmarks
    group.bench_function("convert_borsh_to_json", |b| {
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person).unwrap();
        b.iter(|| {
            let _ = storage.clone().convert(JsonSerializer).unwrap();
        })
    });

    group.bench_function("convert_json_to_wincode", |b| {
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person).unwrap();
        b.iter(|| {
            let _ = storage.clone().convert(WincodeSerializer).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
