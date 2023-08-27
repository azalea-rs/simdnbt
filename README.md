# simdnbt

an unnecessarily fast nbt decoder. like seriously you probably don't need this unless you're trying to win benchmarks.

at the moment, simdnbt does not actually make use of simd instructions (the name is a parody of simdjson). there's one place where i know i could take advantage of simd but it just hasn't been implemented yet (swapping the endianness of integer arrays).

simdnbt might be the fastest nbt decoder currently in existence. however to achieve this silly speed, it takes a couple of shortcuts:
1. it requires a reference to the original data (to avoid cloning)
2. it doesn't validate/decode the mutf-8 strings at decode-time

here's a benchmark with the two other fastest nbt crates (one of which was also made by me):
![simdnbt is ~3x faster than the second fastest nbt crate](https://github.com/mat-1/simdnbt/assets/27899617/4800c672-2800-4b7d-a56a-dcd57d70f094)
Take this with a grain of salt as they're not all doing the same work.
