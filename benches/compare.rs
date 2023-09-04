use std::{
    fs::File,
    io::{Cursor, Read},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use flate2::read::GzDecoder;

pub fn bench_read_file(filename: &str, c: &mut Criterion) {
    let mut file = File::open(format!("tests/{filename}")).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();
    let mut src = &contents[..];

    // decode the original src so most of the time isn't spent on unzipping
    let mut decoded_src_decoder = GzDecoder::new(&mut src);
    let mut input = Vec::new();
    if decoded_src_decoder.read_to_end(&mut input).is_err() {
        // oh probably wasn't gzipped then
        input = contents;
    }
    let input = input.as_slice();

    let mut group = c.benchmark_group(format!("compare/{filename}"));
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("azalea_parse", |b| {
        b.iter(|| {
            let input = black_box(input);
            let nbt = azalea_nbt::Nbt::read(&mut Cursor::new(input)).unwrap();
            black_box(nbt);
        })
    });

    group.bench_function("graphite_parse", |b| {
        b.iter(|| {
            let input = black_box(input);
            let nbt = graphite_binary::nbt::decode::read(&mut &input[..]).unwrap();
            black_box(nbt);
        })
    });

    group.bench_function("simdnbt_parse", |b| {
        b.iter(|| {
            let input = black_box(input);
            let nbt = simdnbt::Nbt::new(&mut Cursor::new(input)).unwrap().unwrap();
            let _ = black_box(nbt.list("").unwrap().ints());
            black_box(nbt);
        })
    });

    // group.bench_function("valence_parse", |b| {
    //     b.iter(|| {
    //         let input = black_box(input);
    //         let nbt = valence_nbt::Compound::from_binary(&mut &input[..]).unwrap();
    //         black_box(nbt);
    //     })
    // });

    // group.bench_function("fastnbt_parse", |b| {
    //     b.iter(|| {
    //         let input = black_box(input);
    //         let nbt: fastnbt::Value = fastnbt::from_bytes(input).unwrap();
    //         black_box(nbt);
    //     })
    // });

    // group.bench_function("hematite_parse", |b| {
    //     b.iter(|| {
    //         let input = black_box(input);
    //         let nbt = nbt::Blob::from_reader(&mut Cursor::new(input)).unwrap();
    //         black_box(nbt);
    //     })
    // });
}

fn bench(c: &mut Criterion) {
    // bench_read_file("hello_world.nbt", c);
    // bench_read_file("bigtest.nbt", c);
    // bench_read_file("simple_player.dat", c);
    bench_read_file("complex_player.dat", c);
    bench_read_file("level.dat", c);
    // bench_read_file("stringtest.nbt", c);
    bench_read_file("inttest1023.nbt", c);
}

criterion_group!(compare, bench);
criterion_main!(compare);
