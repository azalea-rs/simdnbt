use std::simd::prelude::*;

pub trait SwappableNumber {}
impl SwappableNumber for u16 {}
impl SwappableNumber for u32 {}
impl SwappableNumber for u64 {}
impl SwappableNumber for i16 {}
impl SwappableNumber for i32 {}
impl SwappableNumber for i64 {}
impl SwappableNumber for f32 {}
impl SwappableNumber for f64 {}

#[inline]
fn swap_endianness_16bit(bytes: &mut [u8], num: usize) {
    for i in 0..num / 32 {
        let simd: u8x64 = Simd::from_slice(bytes[i * 32 * 2..(i + 1) * 32 * 2].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
            3, 2,
            5, 4,
            7, 6,
            9, 8,
            11, 10,
            13, 12,
            15, 14,
            17, 16,
            19, 18,
            21, 20,
            23, 22,
            25, 24,
            27, 26,
            29, 28,
            31, 30,
            33, 32,
            35, 34,
            37, 36,
            39, 38,
            41, 40,
            43, 42,
            45, 44,
            47, 46,
            49, 48,
            51, 50,
            53, 52,
            55, 54,
            57, 56,
            59, 58,
            61, 60,
            63, 62,
        ]);
        bytes[i * 32 * 2..(i + 1) * 32 * 2].copy_from_slice(simd.as_array());
    }

    let mut i = num / 32 * 32;
    if i + 16 <= num {
        let simd: u8x32 = Simd::from_slice(bytes[i * 2..i * 2 + 32].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
            3, 2,
            5, 4,
            7, 6,
            9, 8,
            11, 10,
            13, 12,
            15, 14,
            17, 16,
            19, 18,
            21, 20,
            23, 22,
            25, 24,
            27, 26,
            29, 28,
            31, 30,
        ]);
        bytes[i * 2..i * 2 + 32].copy_from_slice(simd.as_array());
        i += 16;
    }
    if i + 8 <= num {
        let simd: u8x16 = Simd::from_slice(bytes[i * 2..i * 2 + 16].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
            3, 2,
            5, 4,
            7, 6,
            9, 8,
            11, 10,
            13, 12,
            15, 14,
        ]);
        bytes[i * 2..i * 2 + 16].copy_from_slice(simd.as_array());
        i += 8;
    }
    if i + 4 <= num {
        let simd: u8x8 = Simd::from_slice(bytes[i * 2..i * 2 + 8].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
            3, 2,
            5, 4,
            7, 6,
        ]);
        bytes[i * 2..i * 2 + 8].copy_from_slice(simd.as_array());
        i += 4;
    }
    if i + 2 <= num {
        let simd: u8x4 = Simd::from_slice(bytes[i * 2..i * 2 + 4].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
            3, 2,
        ]);
        bytes[i * 2..i * 2 + 4].copy_from_slice(simd.as_array());
        i += 2;
    }
    if i < num {
        let simd: u8x2 = Simd::from_slice(bytes[i * 2..i * 2 + 2].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            1, 0,
        ]);
        bytes[i * 2..i * 2 + 2].copy_from_slice(simd.as_array());
    }
}

#[inline]
fn swap_endianness_32bit(bytes: &mut [u8], num: usize) {
    for i in 0..num / 16 {
        let simd: u8x64 = Simd::from_slice(bytes[i * 16 * 4..(i + 1) * 16 * 4].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            3, 2, 1, 0,
            7, 6, 5, 4,
            11, 10, 9, 8,
            15, 14, 13, 12,
            19, 18, 17, 16,
            23, 22, 21, 20,
            27, 26, 25, 24,
            31, 30, 29, 28,
            35, 34, 33, 32,
            39, 38, 37, 36,
            43, 42, 41, 40,
            47, 46, 45, 44,
            51, 50, 49, 48,
            55, 54, 53, 52,
            59, 58, 57, 56,
            63, 62, 61, 60,
        ]);
        bytes[i * 16 * 4..(i + 1) * 16 * 4].copy_from_slice(simd.as_array());
    }

    let mut i = num / 16 * 16;
    if i + 8 <= num {
        let simd: u8x32 = Simd::from_slice(bytes[i * 4..i * 4 + 32].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            3, 2, 1, 0,
            7, 6, 5, 4,
            11, 10, 9, 8,
            15, 14, 13, 12,
            19, 18, 17, 16,
            23, 22, 21, 20,
            27, 26, 25, 24,
            31, 30, 29, 28,
        ]);
        bytes[i * 4..i * 4 + 32].copy_from_slice(simd.as_array());
        i += 8;
    }
    if i + 4 <= num {
        let simd: u8x16 = Simd::from_slice(bytes[i * 4..i * 4 + 16].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            3, 2, 1, 0,
            7, 6, 5, 4,
            11, 10, 9, 8,
            15, 14, 13, 12,
        ]);
        bytes[i * 4..i * 4 + 16].copy_from_slice(simd.as_array());
        i += 4;
    }
    if i + 2 <= num {
        let simd: u8x8 = Simd::from_slice(bytes[i * 4..i * 4 + 8].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            3, 2, 1, 0,
            7, 6, 5, 4,
        ]);
        bytes[i * 4..i * 4 + 8].copy_from_slice(simd.as_array());
        i += 2;
    }
    if i < num {
        let simd: u8x4 = Simd::from_slice(bytes[i * 4..i * 4 + 4].as_ref());
        #[rustfmt::skip]
        let simd = simd_swizzle!(simd, [
            3, 2, 1, 0,
        ]);
        bytes[i * 4..i * 4 + 4].copy_from_slice(simd.as_array());
    }
}

#[inline]
fn swap_endianness_64bit(bytes: &mut [u8], num: usize) {
    for i in 0..num / 8 {
        let simd: u8x64 = Simd::from_slice(bytes[i * 64..i * 64 + 64].as_ref());
        #[rustfmt::skip]
            let simd = simd_swizzle!(simd, [
                7, 6, 5, 4, 3, 2, 1, 0,
                15, 14, 13, 12, 11, 10, 9, 8,
                23, 22, 21, 20, 19, 18, 17, 16,
                31, 30, 29, 28, 27, 26, 25, 24,
                39, 38, 37, 36, 35, 34, 33, 32,
                47, 46, 45, 44, 43, 42, 41, 40,
                55, 54, 53, 52, 51, 50, 49, 48,
                63, 62, 61, 60, 59, 58, 57, 56,
            ]);
        bytes[i * 64..i * 64 + 64].copy_from_slice(simd.as_array());
    }

    let mut i = num / 8 * 8;
    if i + 4 <= num {
        let simd: u8x32 = Simd::from_slice(bytes[i * 8..i * 8 + 32].as_ref());
        #[rustfmt::skip]
            let simd = simd_swizzle!(simd, [
                7, 6, 5, 4, 3, 2, 1, 0,
                15, 14, 13, 12, 11, 10, 9, 8,
                23, 22, 21, 20, 19, 18, 17, 16,
                31, 30, 29, 28, 27, 26, 25, 24,
            ]);
        bytes[i * 8..i * 8 + 32].copy_from_slice(simd.as_array());
        i += 4;
    }
    if i + 2 <= num {
        let simd: u8x16 = Simd::from_slice(bytes[i * 8..i * 8 + 16].as_ref());
        #[rustfmt::skip]
            let simd = simd_swizzle!(simd, [
                7, 6, 5, 4, 3, 2, 1, 0,
                15, 14, 13, 12, 11, 10, 9, 8,
            ]);
        bytes[i * 8..i * 8 + 16].copy_from_slice(simd.as_array());
        i += 2;
    }
    if i < num {
        let simd: u8x8 = Simd::from_slice(bytes[i * 8..i * 8 + 8].as_ref());
        #[rustfmt::skip]
            let simd = simd_swizzle!(simd, [
                7, 6, 5, 4, 3, 2, 1, 0,
            ]);
        bytes[i * 8..i * 8 + 8].copy_from_slice(simd.as_array());
    }
}

#[inline]
pub fn swap_endianness_as_u8<T: SwappableNumber>(data: &[u8]) -> Vec<u8> {
    let length = data.len() / std::mem::size_of::<T>();

    let mut items = data.to_vec();

    if cfg!(target_endian = "little") {
        match std::mem::size_of::<T>() {
            2 => swap_endianness_16bit(&mut items, length),
            4 => swap_endianness_32bit(&mut items, length),
            8 => swap_endianness_64bit(&mut items, length),
            _ => panic!("unsupported size of type"),
        }
    }

    items
}

#[inline]
pub fn swap_endianness<T: SwappableNumber>(data: &[u8]) -> Vec<T> {
    let length = data.len() / std::mem::size_of::<T>();
    let items = swap_endianness_as_u8::<T>(data);

    {
        let ptr = items.as_ptr() as *const T;
        std::mem::forget(items);
        // SAFETY: The length won't be greater than the length of the original data
        unsafe { Vec::from_raw_parts(ptr as *mut T, length, length) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_endianness_u16() {
        assert_eq!(
            swap_endianness_as_u8::<u16>(&[1, 2, 3, 4, 5, 6, 7, 8]),
            [2, 1, 4, 3, 6, 5, 8, 7]
        );
    }
    #[test]
    fn test_swap_endianness_u32() {
        assert_eq!(
            swap_endianness_as_u8::<u32>(&[1, 2, 3, 4, 5, 6, 7, 8]),
            [4, 3, 2, 1, 8, 7, 6, 5]
        );
    }
    #[test]
    fn test_swap_endianness_u64() {
        assert_eq!(
            swap_endianness_as_u8::<u64>(&[1, 2, 3, 4, 5, 6, 7, 8]),
            [8, 7, 6, 5, 4, 3, 2, 1]
        );
    }
}
