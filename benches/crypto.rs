#[macro_use]
extern crate criterion;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::black_box;
use sodiumoxide::crypto::box_::{self, PublicKey, SecretKey};
use std::io::Cursor;
use std::io::prelude::*;
use brchd::crypto::upload::EncryptedUpload;

fn encrypt_upload(pk: &PublicKey, sk: &SecretKey, len: usize) {
    let bytes = sodiumoxide::randombytes::randombytes(len);

    // encrypt
    let r = Cursor::new(&bytes);
    let mut upload = EncryptedUpload::new(r, &pk, Some(&sk)).unwrap();

    let mut encrypted = Vec::new();
    upload.read_to_end(&mut encrypted).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let (pk, sk) = box_::gen_keypair();

    let mut group = c.benchmark_group("EncryptedUpload");

    for (size, samples) in &[(16, 100), (1024 * 1024 * 16, 10)] {
        group.sample_size(*samples);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &s| {
            b.iter(|| encrypt_upload(&pk, &sk, s));
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
