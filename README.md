# simdnbt

a very fast nbt serializer and deserializer.

simdnbt currently makes use of simd instructions for two things:
- swapping the endianness of int arrays
- checking if a string is plain ascii for faster mutf8 to utf8 conversion

simdnbt might be the fastest nbt decoder currently in existence. however to achieve this silly speed, it takes a couple of shortcuts:
1. it requires a reference to the original data (to avoid cloning)
2. it doesn't validate/decode the mutf-8 strings at decode-time

here's a benchmark comparing simdnbt against a few of the other fastest nbt crates (though without actually accessing the data):

![simdnbt is ~3x faster than the second fastest nbt crate](https://github.com/azalea-rs/simdnbt/assets/27899617/03a4f916-d162-4a23-aa1a-12f1b11dc903)


and here's a benchmark where it accesses the data and makes it owned:

![simdnbt is only about 50% faster than the second fastest in this one](https://github.com/azalea-rs/simdnbt/assets/27899617/9d716c39-3bff-4703-99d7-2bec91c6b205)
