# simdnbt

an unnecessarily fast nbt decoder. like seriously you probably don't need this unless you're trying to win benchmarks.

simdnbt currently only makes use of simd instructions for swapping the endianness of arrays, and tbh that's really only there so i can call it "simdnbt" without lying. the name is mostly a play on simdjson.

simdnbt might be the fastest nbt decoder currently in existence. however to achieve this silly speed, it takes a couple of shortcuts:
1. it requires a reference to the original data (to avoid cloning)
2. it doesn't validate/decode the mutf-8 strings at decode-time

here's a benchmark with the two other fastest nbt crates (azalea-nbt was also made by me):
![simdnbt is ~3x faster than the second fastest nbt crate](https://github.com/mat-1/simdnbt/assets/27899617/4c252b98-628c-4d81-92cd-3c8e1a7bd023)

take this with a grain of salt as they're not all doing the same work. regardless, you can still see it's very fast.
