# simdnbt

Simdnbt is a very fast [NBT](https://minecraft.wiki/w/NBT_format) serializer and deserializer.

It was originally made as a joke but it ended up being too good of a joke so it's actually a thing now.

## Usage

```sh
cargo add simdnbt
```

### Deserializing

For deserializing, you'll likely want either [simdnbt::borrow::Nbt::read](https://docs.rs/simdnbt/latest/simdnbt/borrow/enum.Nbt.html#method.read) or [simdnbt::owned::Nbt::read](https://docs.rs/simdnbt/latest/simdnbt/owned/enum.Nbt.html#method.read).
The difference is that the "borrow" variant requires you to keep a reference to the original buffer, but is significantly faster.

```rust,no_run
use std::borrow::Cow;
use std::io::Cursor;
use simdnbt::borrow::{cursor::McCursor, Nbt};

fn example(item_bytes: &[u8]) {
    let nbt = Nbt::read(&mut McCursor::new(item_bytes))
        .unwrap()
        .unwrap();
    let skyblock_id: Cow<str> = nbt
        .list("i")
        .and_then(|i| i.compounds())
        .and_then(|i| i.get(0))
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

The most significant and simple optimization you can do is switching to an allocator like [mimalloc](https://docs.rs/mimalloc/latest/mimalloc/) (it's ~20% faster on my machine). Setting `RUSTFLAGS='-C target-cpu=native'` when running your code may also help a little bit.

## Implementation details

Simdnbt currently makes use of SIMD instructions for two things:
- swapping the endianness of int arrays
- checking if a string is plain ascii for faster mutf8 to utf8 conversion

Simdnbt ~~cheats~~ takes some shortcuts to be this fast:
1. it requires a reference to the original data (to avoid cloning)
2. it doesn't validate/decode the mutf-8 strings at decode-time

## Benchmarks

Simdnbt is likely the fastest NBT decoder currently in existence.

Here's a benchmark comparing Simdnbt against a few of the other fastest NBT crates (though without actually accessing the data):

![simdnbt is ~3x faster than the second fastest nbt crate](https://github.com/azalea-rs/simdnbt/assets/27899617/03a4f916-d162-4a23-aa1a-12f1b11dc903)

And here's a benchmark where it accesses the data and makes it owned:

![simdnbt is only about 50% faster than the second fastest in this one](https://github.com/azalea-rs/simdnbt/assets/27899617/9d716c39-3bff-4703-99d7-2bec91c6b205)
