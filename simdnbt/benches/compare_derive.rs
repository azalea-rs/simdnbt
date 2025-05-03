use std::{
    fs::File,
    io::{Cursor, Read},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use flate2::read::GzDecoder;
use simdnbt::{Deserialize, Mutf8Str};

fn bench_read_file(filename: &str, c: &mut Criterion) {
    let mut file = File::open(format!("tests/{filename}")).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();
    let mut src = &contents[..];

    // decode the original src so most of the time isn't spent on unzipping
    let mut src_decoder = GzDecoder::new(&mut src);
    let mut input = Vec::new();
    if src_decoder.read_to_end(&mut input).is_err() {
        // oh probably wasn't gzipped then
        input = contents;
    }

    let mut input_stream = Cursor::new(&input[..]);

    let mut group = c.benchmark_group(format!("compare_derive/{filename}"));
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("simdnbt_owned_parse", |b| {
        b.iter(|| {
            black_box(simdnbt::owned::read(&mut input_stream).unwrap());
            input_stream.set_position(0);
        })
    });
    group.bench_function("simdnbt_borrow_parse", |b| {
        b.iter(|| {
            black_box(simdnbt::borrow::read(&mut input_stream).unwrap());
            input_stream.set_position(0);
        })
    });
    group.bench_function("simdnbt_validate_parse", |b| {
        b.iter(|| {
            simdnbt::validate::NbtValidator::new()
                .read(&mut input_stream)
                .unwrap();
            input_stream.set_position(0);
        })
    });
    group.bench_function("simdnbt_derive_parse", |b| {
        b.iter(|| {
            black_box(Base::read(&mut input_stream).unwrap());
            input_stream.set_position(0);
        })
    });
}

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn bench(c: &mut Criterion) {
    bench_read_file("hypixel.nbt", c);
}

criterion_group!(compare, bench);
criterion_main!(compare);

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Item<'a> {
    pub id: i16,
    #[simdnbt(rename = "Damage")]
    pub damage: Option<i16>,
    #[simdnbt(rename = "Count")]
    pub count: i8,

    pub tag: ItemTag<'a>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ItemTag<'a> {
    #[simdnbt(rename = "SkullOwner")]
    pub skull_owner: Option<Box<SkullOwner<'a>>>,
    #[simdnbt(rename = "ExtraAttributes")]
    pub extra_attributes: Option<Box<ExtraAttributes<'a>>>,
    pub display: Option<Box<ItemDisplay<'a>>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExtraAttributes<'a> {
    pub id: Option<&'a Mutf8Str>,
    pub modifier: Option<&'a Mutf8Str>,

    // pub ench: Option<simdnbt::owned::NbtCompound>,
    // pub enchantments: Option<HashMap<&'a Mutf8Str, i32>>,
    pub enchantments: Option<Vec<(&'a Mutf8Str, i32)>>,
    pub timestamp: Option<&'a Mutf8Str>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SkullOwner<'a> {
    #[simdnbt(rename = "Properties")]
    pub properties: Properties<'a>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Properties<'a> {
    pub textures: Vec<Texture<'a>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Texture<'a> {
    #[simdnbt(rename = "Value")]
    pub value: &'a Mutf8Str,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ItemDisplay<'a> {
    #[simdnbt(rename = "Name")]
    pub name: &'a Mutf8Str,
    #[simdnbt(rename = "Lore")]
    pub lore: Vec<&'a Mutf8Str>,

    pub color: Option<i32>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Base<'a> {
    #[simdnbt(rename = "i")]
    pub items: Vec<Option<Item<'a>>>,
}
