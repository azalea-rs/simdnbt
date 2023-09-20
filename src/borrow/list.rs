use std::{io::Cursor, marker::PhantomData, simd::prelude::*, slice};

use byteorder::ReadBytesExt;

use crate::{
    common::{
        BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID,
        LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    Error, Mutf8Str,
};

use super::{read_string, read_u32, read_with_u32_length, CompoundTag, MAX_DEPTH};

/// A list of NBT tags of a single type.
#[derive(Debug, Default)]
pub enum ListTag<'a> {
    #[default]
    Empty,
    Byte(&'a [i8]),
    Short(RawList<'a, i16>),
    Int(RawList<'a, i32>),
    Long(RawList<'a, i64>),
    Float(RawList<'a, f32>),
    Double(RawList<'a, f64>),
    ByteArray(&'a [u8]),
    String(Vec<&'a Mutf8Str>),
    List(Vec<ListTag<'a>>),
    Compound(Vec<CompoundTag<'a>>),
    IntArray(Vec<Vec<i32>>),
    LongArray(Vec<Vec<i64>>),
}
impl<'a> ListTag<'a> {
    pub fn new(data: &mut Cursor<&'a [u8]>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        Ok(match tag_type {
            END_ID => {
                data.set_position(data.position() + 4);
                ListTag::Empty
            }
            BYTE_ID => ListTag::Byte(read_i8_array(data)?),
            SHORT_ID => ListTag::Short(RawList::new(read_with_u32_length(data, 2)?)),
            INT_ID => ListTag::Int(RawList::new(read_with_u32_length(data, 4)?)),
            LONG_ID => ListTag::Long(RawList::new(read_with_u32_length(data, 8)?)),
            FLOAT_ID => ListTag::Float(RawList::new(read_with_u32_length(data, 4)?)),
            DOUBLE_ID => ListTag::Double(RawList::new(read_with_u32_length(data, 8)?)),
            BYTE_ARRAY_ID => ListTag::ByteArray(read_u8_array(data)?),
            STRING_ID => ListTag::String({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut strings = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    strings.push(read_string(data)?)
                }
                strings
            }),
            LIST_ID => ListTag::List({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut lists = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    lists.push(ListTag::new(data, depth + 1)?)
                }
                lists
            }),
            COMPOUND_ID => ListTag::Compound({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut compounds = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    compounds.push(CompoundTag::new(data, depth + 1)?)
                }
                compounds
            }),
            INT_ARRAY_ID => ListTag::IntArray({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(read_int_array(data)?)
                }
                arrays
            }),
            LONG_ARRAY_ID => ListTag::LongArray({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(read_long_array(data)?)
                }
                arrays
            }),
            _ => return Err(Error::UnknownTagId(tag_type)),
        })
    }

    pub fn bytes(&self) -> Option<&[i8]> {
        match self {
            ListTag::Byte(bytes) => Some(bytes),
            _ => None,
        }
    }
    pub fn shorts(&self) -> Option<Vec<i16>> {
        match self {
            ListTag::Short(shorts) => Some(shorts.to_vec()),
            _ => None,
        }
    }
    pub fn ints(&self) -> Option<Vec<i32>> {
        match self {
            ListTag::Int(ints) => Some(ints.to_vec()),
            _ => None,
        }
    }
    pub fn longs(&self) -> Option<Vec<i64>> {
        match self {
            ListTag::Long(longs) => Some(longs.to_vec()),
            _ => None,
        }
    }
    pub fn floats(&self) -> Option<Vec<f32>> {
        match self {
            ListTag::Float(floats) => Some(floats.to_vec()),
            _ => None,
        }
    }
    pub fn doubles(&self) -> Option<Vec<f64>> {
        match self {
            ListTag::Double(doubles) => Some(doubles.to_vec()),
            _ => None,
        }
    }
    pub fn byte_arrays(&self) -> Option<&[u8]> {
        match self {
            ListTag::ByteArray(byte_arrays) => Some(byte_arrays),
            _ => None,
        }
    }
    pub fn strings(&self) -> Option<&[&Mutf8Str]> {
        match self {
            ListTag::String(strings) => Some(strings),
            _ => None,
        }
    }
    pub fn lists(&self) -> Option<&[ListTag]> {
        match self {
            ListTag::List(lists) => Some(lists),
            _ => None,
        }
    }
    pub fn compounds(&self) -> Option<&[CompoundTag]> {
        match self {
            ListTag::Compound(compounds) => Some(compounds),
            _ => None,
        }
    }
    pub fn int_arrays(&self) -> Option<&[Vec<i32>]> {
        match self {
            ListTag::IntArray(int_arrays) => Some(int_arrays),
            _ => None,
        }
    }
    pub fn long_arrays(&self) -> Option<&[Vec<i64>]> {
        match self {
            ListTag::LongArray(long_arrays) => Some(long_arrays),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct RawList<'a, T> {
    data: &'a [u8],
    _marker: PhantomData<T>,
}
impl<'a, T> RawList<'a, T> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            _marker: PhantomData,
        }
    }
}

impl<T> RawList<'_, T> {
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Copy,
    {
        let data = self.data;
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
}

fn read_u8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    read_with_u32_length(data, 1)
}
fn read_i8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [i8], Error> {
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
