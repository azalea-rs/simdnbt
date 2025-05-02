# simdnbt

Simdnbt is a very fast [NBT](https://minecraft.wiki/w/NBT_format) serializer and deserializer.

It was originally made as a joke but it ended up being too good of a joke so it's actually a thing now.

## Usage

```sh
cargo add simdnbt
```

### Deserializing

For deserializing, you'll likely want either [simdnbt::borrow::read](https://docs.rs/simdnbt/latest/simdnbt/borrow/fn.read.html) or [simdnbt::owned::read](https://docs.rs/simdnbt/latest/simdnbt/owned/fn.read.html).
The difference is that the "borrow" variant requires you to keep a reference to the original buffer, but is significantly faster.

```rust,no_run
use std::borrow::Cow;
use std::io::Cursor;

fn example(item_bytes: &[u8]) {
    let nbt = simdnbt::borrow::read(&mut Cursor::new(item_bytes))
        .unwrap()
        .unwrap();
    let skyblock_id: Cow<str> = nbt
        .list("i")
        .and_then(|i| i.compounds())
        .and_then(|i| i.first())
        .and_then(|i| i.compound("tag"))
        .and_then(|tag| tag.compound("ExtraAttributes"))
        .and_then(|ea| ea.string("id"))
        .map(|id| id.to_string_lossy())
        .unwrap_or_default();
}
```

### Serializing

```rust
use simdnbt::owned::{BaseNbt, Nbt, NbtCompound, NbtTag};

let nbt = Nbt::Some(BaseNbt::new(
    "",
    NbtCompound::from_values(vec![
        ("key".into(), NbtTag::String("value".into())),
    ]),
));
let mut buffer = Vec::new();
nbt.write(&mut buffer);
```

## Performance guide

Use the borrow variant of `Nbt` if possible, and avoid allocating unnecessarily (for example, keep strings as `Cow<str>` if you can).

If you're using the owned variant of Simdnbt, switching to a faster allocator like [mimalloc](https://docs.rs/mimalloc/latest/mimalloc/) may help a decent amount (it's ~20% faster on my machine).

Using `RUSTFLAGS="-Cllvm-args=-enable-dfa-jump-thread"` makes Simdnbt about 4% faster in exchange for potentially slightly longer compile times and compiler instability.

Setting `RUSTFLAGS='-C target-cpu=native'` when running your code may help or hurt performance, depending on your computer and program.

## Implementation details

The "SIMD" part of the name is there as a reference to simdjson, and isn't usually critical to Simdnbt's decoding speed. Regardless, Simdnbt does actually make use of SIMD instructions for two things:

- swapping the endianness of int arrays.
- checking if a string is plain ascii for faster MUTF-8 to UTF-8 conversion.

Additionally, Simdnbt takes some shortcuts which usually aren't taken by other libraries:

- `simdnbt::borrow` requires a reference to the original data.
- it doesn't validate/decode MUTF-8 strings or integer arrays while parsing.
- compounds aren't sorted, so lookup always does a linear search.

Several ideas are borrowed from simdjson, notably the usage of a [tape](https://github.com/simdjson/simdjson/blob/master/doc/tape.md).

## Benchmarks

Simdnbt is the fastest NBT parser in Rust.

Here's a benchmark comparing Simdnbt against a few of the other fastest NBT crates for decoding [`complex_player.dat`](https://github.com/azalea-rs/simdnbt/blob/master/simdnbt/tests/complex_player.dat):

| Library                                                                        | Throughput   |
| ------------------------------------------------------------------------------ | ------------ |
| [simdnbt::borrow](https://docs.rs/simdnbt/latest/simdnbt/borrow/index.html)    | 4.3000 GiB/s |
| [ussr_nbt::borrow](https://docs.rs/ussr-nbt/latest/ussr_nbt/borrow/index.html) | 1.2167 GiB/s |
| [simdnbt::owned](https://docs.rs/simdnbt/latest/simdnbt/owned/index.html)      | 828.09 MiB/s |
| [shen_nbt5](https://docs.rs/shen-nbt5/latest/shen_nbt5/)                       | 540.46 MiB/s |
| [graphite_binary](https://docs.rs/graphite_binary/latest/graphite_binary/)     | 333.47 MiB/s |
| [azalea_nbt](https://docs.rs/azalea-nbt/latest/azalea_nbt/)                    | 328.62 MiB/s |
| [valence_nbt](https://docs.rs/valence_nbt/latest/valence_nbt/)                 | 275.88 MiB/s |
| [crab_nbt](https://docs.rs/crab_nbt/latest/crab_nbt/)                          | 223.31 MiB/s |
| [hematite_nbt](https://docs.rs/hematite-nbt/latest/nbt/)                       | 161.77 MiB/s |
| [fastnbt](https://docs.rs/fastnbt/latest/fastnbt/)                             | 160.98 MiB/s |

And for writing `complex_player.dat`:

| Library         | Throughput   |
| --------------- | ------------ |
| simdnbt::owned  | 2.6633 GiB/s |
| simdnbt::borrow | 2.4228 GiB/s |
| azalea_nbt      | 2.1755 GiB/s |
| graphite_binary | 1.8010 GiB/s |

The tables above were made from the [compare benchmark](https://github.com/azalea-rs/simdnbt/tree/master/simdnbt/benches) in this repo, with `cargo bench 'compare/complex_player.dat/'`.

Note that the benchmark is somewhat unfair, since Simdnbt takes a few shortcuts that other libraries don't. See the Implementation Details section above for more info.

Also keep in mind that if you run your own benchmark you'll get different numbers, but the speeds should be about the same relative to each other.
