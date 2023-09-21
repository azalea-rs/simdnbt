use std::io::Cursor;

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_i8_array, read_int_array, read_long_array, read_string, read_u8_array,
        read_with_u32_length, slice_i8_into_u8, slice_into_u8_big_endian, unchecked_extend,
        unchecked_push, write_string, write_u32, write_with_u32_length, BYTE_ARRAY_ID, BYTE_ID,
        COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID,
        LONG_ID, SHORT_ID, STRING_ID,
    },
    mutf8::Mutf8String,
    swap_endianness::swap_endianness,
    Error,
};

use super::{compound::CompoundTag, read_u32, MAX_DEPTH};

/// A list of NBT tags of a single type.
#[repr(u8)]
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ListTag {
    #[default]
    Empty = END_ID,
    Byte(Vec<i8>) = BYTE_ID,
    Short(Vec<i16>) = SHORT_ID,
    Int(Vec<i32>) = INT_ID,
    Long(Vec<i64>) = LONG_ID,
    Float(Vec<f32>) = FLOAT_ID,
    Double(Vec<f64>) = DOUBLE_ID,
    ByteArray(Vec<Vec<u8>>) = BYTE_ARRAY_ID,
    String(Vec<Mutf8String>) = STRING_ID,
    List(Vec<ListTag>) = LIST_ID,
    Compound(Vec<CompoundTag>) = COMPOUND_ID,
    IntArray(Vec<Vec<i32>>) = INT_ARRAY_ID,
    LongArray(Vec<Vec<i64>>) = LONG_ARRAY_ID,
}
impl ListTag {
    pub fn new(data: &mut Cursor<&[u8]>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        Ok(match tag_type {
            END_ID => {
                data.set_position(data.position() + 4);
                ListTag::Empty
            }
            BYTE_ID => ListTag::Byte(read_i8_array(data)?.to_owned()),
            SHORT_ID => ListTag::Short(swap_endianness(read_with_u32_length(data, 2)?)),
            INT_ID => ListTag::Int(swap_endianness(read_with_u32_length(data, 4)?)),
            LONG_ID => ListTag::Long(swap_endianness(read_with_u32_length(data, 8)?)),
            FLOAT_ID => ListTag::Float(swap_endianness(read_with_u32_length(data, 4)?)),
            DOUBLE_ID => ListTag::Double(swap_endianness(read_with_u32_length(data, 8)?)),
            BYTE_ARRAY_ID => ListTag::ByteArray({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(read_u8_array(data)?.to_vec())
                }
                arrays
            }),
            STRING_ID => ListTag::String({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut strings = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    strings.push(read_string(data)?.to_owned())
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
                    arrays.push(read_int_array(data)?.to_vec())
                }
                arrays
            }),
            LONG_ARRAY_ID => ListTag::LongArray({
                let length = read_u32(data)?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(read_long_array(data)?.to_vec())
                }
                arrays
            }),
            _ => return Err(Error::UnknownTagId(tag_type)),
        })
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        // fast path for compound since it's very common to have lists of compounds
        if let ListTag::Compound(compounds) = self {
            data.reserve(5);
            // SAFETY: we just reserved 5 bytes
            unsafe {
                unchecked_push(data, COMPOUND_ID);
                unchecked_extend(data, &(compounds.len() as u32).to_be_bytes());
            }
            for compound in compounds {
                compound.write(data);
            }
            return;
        }

        data.push(self.id());
        match self {
            ListTag::Empty => {
                data.extend(&0u32.to_be_bytes());
            }
            ListTag::Byte(bytes) => {
                write_with_u32_length(data, 1, slice_i8_into_u8(bytes));
            }
            ListTag::Short(shorts) => {
                write_with_u32_length(data, 2, &slice_into_u8_big_endian(shorts));
            }
            ListTag::Int(ints) => {
                write_with_u32_length(data, 4, &slice_into_u8_big_endian(ints));
            }
            ListTag::Long(longs) => {
                write_with_u32_length(data, 8, &slice_into_u8_big_endian(longs));
            }
            ListTag::Float(floats) => {
                write_with_u32_length(data, 4, &slice_into_u8_big_endian(floats));
            }
            ListTag::Double(doubles) => {
                write_with_u32_length(data, 8, &slice_into_u8_big_endian(doubles));
            }
            ListTag::ByteArray(byte_arrays) => {
                write_u32(data, byte_arrays.len() as u32);
                for array in byte_arrays {
                    write_with_u32_length(data, 1, array);
                }
            }
            ListTag::String(strings) => {
                write_u32(data, strings.len() as u32);
                for string in strings {
                    write_string(data, string);
                }
            }
            ListTag::List(lists) => {
                write_u32(data, lists.len() as u32);
                for list in lists {
                    list.write(data);
                }
            }
            ListTag::Compound(_) => {
                unreachable!("fast path for compound should have been taken")
            }
            ListTag::IntArray(int_arrays) => {
                write_u32(data, int_arrays.len() as u32);
                for array in int_arrays {
                    write_with_u32_length(data, 4, &slice_into_u8_big_endian(array));
                }
            }
            ListTag::LongArray(long_arrays) => {
                write_u32(data, long_arrays.len() as u32);
                for array in long_arrays {
                    write_with_u32_length(data, 8, &slice_into_u8_big_endian(array));
                }
            }
        }
    }

    /// Get the numerical ID of the tag type.
    #[inline]
    pub fn id(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)`
        // `union` between `repr(C)` structs, each of which has the `u8`
        // discriminant as its first field, so we can read the discriminant
        // without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
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
    pub fn byte_arrays(&self) -> Option<&[Vec<u8>]> {
        match self {
            ListTag::ByteArray(byte_arrays) => Some(byte_arrays),
            _ => None,
        }
    }
    pub fn strings(&self) -> Option<&[Mutf8String]> {
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
