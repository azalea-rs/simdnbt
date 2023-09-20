use std::{
    io::Cursor,
    simd::{simd_swizzle, u8x16, u8x32, u8x4, u8x64, u8x8, Simd},
    slice,
};

use crate::{Error, Mutf8Str};

pub const END_ID: u8 = 0;
pub const BYTE_ID: u8 = 1;
pub const SHORT_ID: u8 = 2;
pub const INT_ID: u8 = 3;
pub const LONG_ID: u8 = 4;
pub const FLOAT_ID: u8 = 5;
pub const DOUBLE_ID: u8 = 6;
pub const BYTE_ARRAY_ID: u8 = 7;
pub const STRING_ID: u8 = 8;
pub const LIST_ID: u8 = 9;
pub const COMPOUND_ID: u8 = 10;
pub const INT_ARRAY_ID: u8 = 11;
pub const LONG_ARRAY_ID: u8 = 12;

pub const MAX_DEPTH: usize = 512;

#[inline(always)]
pub fn read_u32(data: &mut Cursor<&[u8]>) -> Result<u32, Error> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 4 {
        return Err(Error::UnexpectedEof);
    }

    data.set_position(data.position() + 4);

    Ok(u32::from_be_bytes([
        remaining_slice[0],
        remaining_slice[1],
        remaining_slice[2],
        remaining_slice[3],
    ]))
}
#[inline(always)]
pub fn read_u16(data: &mut Cursor<&[u8]>) -> Result<u16, Error> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 2 {
        return Err(Error::UnexpectedEof);
    }

    data.set_position(data.position() + 2);

    Ok(u16::from_be_bytes([remaining_slice[0], remaining_slice[1]]))
}

#[inline(always)]
pub fn read_with_u16_length<'a>(
    data: &mut Cursor<&'a [u8]>,
    width: usize,
) -> Result<&'a [u8], Error> {
    let length = read_u16(data)?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(Error::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

#[inline(never)]
pub fn read_with_u32_length<'a>(
    data: &mut Cursor<&'a [u8]>,
    width: usize,
) -> Result<&'a [u8], Error> {
    let length = read_u32(data)?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(Error::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

pub fn read_string<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a Mutf8Str, Error> {
    let data = read_with_u16_length(data, 1)?;
    Ok(Mutf8Str::from_slice(data))
}

pub fn read_u8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    read_with_u32_length(data, 1)
}
pub fn read_i8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [i8], Error> {
    Ok(slice_u8_into_i8(read_u8_array(data)?))
}
pub fn read_int_array(data: &mut Cursor<&[u8]>) -> Result<Vec<i32>, Error> {
    let array_bytes = read_with_u32_length(data, 4)?;
    let length = array_bytes.len() / 4;
    let mut ints = array_bytes.to_vec();

    if cfg!(target_endian = "little") {
        swap_endianness_32bit(&mut ints, length);
    }

    let ints = {
        let ptr = ints.as_ptr() as *const i32;
        std::mem::forget(ints);
        // SAFETY: the width provided to read_with_u32_length guarantees that it'll be a multiple of 4
        unsafe { Vec::from_raw_parts(ptr as *mut i32, length, length) }
    };

    Ok(ints)
}

pub fn read_long_array(data: &mut Cursor<&[u8]>) -> Result<Vec<i64>, Error> {
    let array_bytes = read_with_u32_length(data, 8)?;
    let length = array_bytes.len() / 8;
    let mut ints = array_bytes.to_vec();

    if cfg!(target_endian = "little") {
        swap_endianness_64bit(&mut ints, length);
    }

    let ints = {
        let ptr = ints.as_ptr() as *const i64;
        std::mem::forget(ints);
        // SAFETY: the width provided to read_with_u32_length guarantees that it'll be a multiple of 8
        unsafe { Vec::from_raw_parts(ptr as *mut i64, length, length) }
    };

    Ok(ints)
}

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

fn slice_u8_into_i8(s: &[u8]) -> &[i8] {
    unsafe { slice::from_raw_parts(s.as_ptr() as *const i8, s.len()) }
}

#[inline]
pub fn swap_endianness<T>(data: &[u8]) -> Vec<T> {
    let length = data.len() / std::mem::size_of::<T>();

    let mut items = data.to_vec();

    if cfg!(target_endian = "little") {
        match std::mem::size_of::<T>() {
            4 => swap_endianness_32bit(&mut items, length),
            8 => swap_endianness_64bit(&mut items, length),
            _ => panic!("unsupported size of type"),
        }
    }

    {
        let ptr = items.as_ptr() as *const T;
        std::mem::forget(items);
        // SAFETY: the width provided to read_with_u32_length guarantees that it'll be a multiple of 4
        unsafe { Vec::from_raw_parts(ptr as *mut T, length, length) }
    }
}
