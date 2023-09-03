use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use simdnbt::Mutf8Str;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("mutf8"));

    group.bench_function("to_str", |b| {
        let input = black_box(Mutf8Str::from_slice(b"hello world"));
        b.iter(|| {
            black_box(input.to_str());
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
