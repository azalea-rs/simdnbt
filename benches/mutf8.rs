use azalea_nbt::Nbt;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{Cursor, Read},
};

fn bench(c: &mut Criterion) {}

criterion_group!(benches, bench);
criterion_main!(benches);
