use std::io::Cursor;

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_i8_array, read_int_array, read_long_array, read_string, read_u8_array,
        read_with_u32_length, slice_i8_into_u8, unchecked_extend, unchecked_push, write_string,
        write_u32, write_with_u32_length, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID,
        FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    raw_list::RawList,
    Error, Mutf8Str,
};

use super::{cursor::McCursor, read_u32, NbtCompound, MAX_DEPTH};

/// A list of NBT tags of a single type.
#[repr(u8)]
#[derive(Debug, Default, PartialEq, Clone)]
pub enum NbtList<'a> {
    #[default]
    Empty = END_ID,
    Byte(&'a [i8]) = BYTE_ID,
    Short(RawList<'a, i16>) = SHORT_ID,
    Int(RawList<'a, i32>) = INT_ID,
    Long(RawList<'a, i64>) = LONG_ID,
    Float(RawList<'a, f32>) = FLOAT_ID,
    Double(RawList<'a, f64>) = DOUBLE_ID,
    ByteArray(Vec<&'a [u8]>) = BYTE_ARRAY_ID,
    String(Vec<&'a Mutf8Str>) = STRING_ID,
    List(Vec<NbtList<'a>>) = LIST_ID,
    Compound(Vec<NbtCompound<'a>>) = COMPOUND_ID,
    IntArray(Vec<RawList<'a, i32>>) = INT_ARRAY_ID,
    LongArray(Vec<RawList<'a, i64>>) = LONG_ARRAY_ID,
}
impl<'a> NbtList<'a> {
    pub fn read(data: &mut McCursor<'a>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        Ok(match tag_type {
            END_ID => {
                data.advance(4);
                NbtList::Empty
            }
            BYTE_ID => NbtList::Byte(data.read_i8_array()?),
            SHORT_ID => NbtList::Short(RawList::new(data.read_with_u32_length(2)?)),
            INT_ID => NbtList::Int(RawList::new(data.read_with_u32_length(4)?)),
            LONG_ID => NbtList::Long(RawList::new(data.read_with_u32_length(8)?)),
            FLOAT_ID => NbtList::Float(RawList::new(data.read_with_u32_length(4)?)),
            DOUBLE_ID => NbtList::Double(RawList::new(data.read_with_u32_length(8)?)),
            BYTE_ARRAY_ID => NbtList::ByteArray({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(data.read_u8_array()?)
                }
                arrays
            }),
            STRING_ID => NbtList::String({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut strings = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    strings.push(data.read_string()?)
                }
                strings
            }),
            LIST_ID => NbtList::List({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut lists = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    lists.push(NbtList::read(data, depth + 1)?)
                }
                lists
            }),
            COMPOUND_ID => NbtList::Compound({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut compounds = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    compounds.push(NbtCompound::read_with_depth(data, depth + 1)?)
                }
                compounds
            }),
            INT_ARRAY_ID => NbtList::IntArray({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(data.read_int_array()?)
                }
                arrays
            }),
            LONG_ARRAY_ID => NbtList::LongArray({
                let length = data.read_u32()?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(data.read_long_array()?)
                }
                arrays
            }),
            _ => return Err(Error::UnknownTagId(tag_type)),
        })
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        // fast path for compound since it's very common to have lists of compounds
        if let NbtList::Compound(compounds) = self {
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
            NbtList::Empty => {
                data.extend(&0u32.to_be_bytes());
            }
            NbtList::Byte(bytes) => {
                write_with_u32_length(data, 1, slice_i8_into_u8(bytes));
            }
            NbtList::Short(shorts) => {
                write_with_u32_length(data, 2, shorts.as_big_endian());
            }
            NbtList::Int(ints) => {
                write_with_u32_length(data, 4, ints.as_big_endian());
            }
            NbtList::Long(longs) => {
                write_with_u32_length(data, 8, longs.as_big_endian());
            }
            NbtList::Float(floats) => {
                write_with_u32_length(data, 4, floats.as_big_endian());
            }
            NbtList::Double(doubles) => {
                write_with_u32_length(data, 8, doubles.as_big_endian());
            }
            NbtList::ByteArray(byte_arrays) => {
                write_u32(data, byte_arrays.len() as u32);
                for array in byte_arrays.iter() {
                    write_with_u32_length(data, 1, array);
                }
            }
            NbtList::String(strings) => {
                write_u32(data, strings.len() as u32);
                for string in strings {
                    write_string(data, string);
                }
            }
            NbtList::List(lists) => {
                write_u32(data, lists.len() as u32);
                for list in lists {
                    list.write(data);
                }
            }
            NbtList::Compound(_) => {
                unreachable!("fast path for compound should have been taken")
            }
            NbtList::IntArray(int_arrays) => {
                write_u32(data, int_arrays.len() as u32);
                for array in int_arrays {
                    write_with_u32_length(data, 4, array.as_big_endian());
                }
            }
            NbtList::LongArray(long_arrays) => {
                write_u32(data, long_arrays.len() as u32);
                for array in long_arrays {
                    write_with_u32_length(data, 8, array.as_big_endian());
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
            NbtList::Byte(bytes) => Some(bytes),
            _ => None,
        }
    }
    pub fn shorts(&self) -> Option<Vec<i16>> {
        match self {
            NbtList::Short(shorts) => Some(shorts.to_vec()),
            _ => None,
        }
    }
    pub fn ints(&self) -> Option<Vec<i32>> {
        match self {
            NbtList::Int(ints) => Some(ints.to_vec()),
            _ => None,
        }
    }
    pub fn longs(&self) -> Option<Vec<i64>> {
        match self {
            NbtList::Long(longs) => Some(longs.to_vec()),
            _ => None,
        }
    }
    pub fn floats(&self) -> Option<Vec<f32>> {
        match self {
            NbtList::Float(floats) => Some(floats.to_vec()),
            _ => None,
        }
    }
    pub fn doubles(&self) -> Option<Vec<f64>> {
        match self {
            NbtList::Double(doubles) => Some(doubles.to_vec()),
            _ => None,
        }
    }
    pub fn byte_arrays(&self) -> Option<&Vec<&[u8]>> {
        match self {
            NbtList::ByteArray(byte_arrays) => Some(byte_arrays),
            _ => None,
        }
    }
    pub fn strings(&self) -> Option<&[&Mutf8Str]> {
        match self {
            NbtList::String(strings) => Some(strings),
            _ => None,
        }
    }
    pub fn lists(&self) -> Option<&[NbtList]> {
        match self {
            NbtList::List(lists) => Some(lists),
            _ => None,
        }
    }
    pub fn compounds(&self) -> Option<&[NbtCompound]> {
        match self {
            NbtList::Compound(compounds) => Some(compounds),
            _ => None,
        }
    }
    pub fn int_arrays(&self) -> Option<&[RawList<i32>]> {
        match self {
            NbtList::IntArray(int_arrays) => Some(int_arrays),
            _ => None,
        }
    }
    pub fn long_arrays(&self) -> Option<&[RawList<i64>]> {
        match self {
            NbtList::LongArray(long_arrays) => Some(long_arrays),
            _ => None,
        }
    }

    pub fn to_owned(&self) -> crate::owned::NbtList {
        match self {
            NbtList::Empty => crate::owned::NbtList::Empty,
            NbtList::Byte(bytes) => crate::owned::NbtList::Byte(bytes.to_vec()),
            NbtList::Short(shorts) => crate::owned::NbtList::Short(shorts.to_vec()),
            NbtList::Int(ints) => crate::owned::NbtList::Int(ints.to_vec()),
            NbtList::Long(longs) => crate::owned::NbtList::Long(longs.to_vec()),
            NbtList::Float(floats) => crate::owned::NbtList::Float(floats.to_vec()),
            NbtList::Double(doubles) => crate::owned::NbtList::Double(doubles.to_vec()),
            NbtList::ByteArray(byte_arrays) => crate::owned::NbtList::ByteArray(
                byte_arrays.iter().map(|array| array.to_vec()).collect(),
            ),
            NbtList::String(strings) => crate::owned::NbtList::String(
                strings.iter().map(|&string| string.to_owned()).collect(),
            ),
            NbtList::List(lists) => {
                crate::owned::NbtList::List(lists.iter().map(|list| list.to_owned()).collect())
            }
            NbtList::Compound(compounds) => crate::owned::NbtList::Compound(
                compounds
                    .iter()
                    .map(|compound| compound.to_owned())
                    .collect(),
            ),
            NbtList::IntArray(int_arrays) => crate::owned::NbtList::IntArray(
                int_arrays
                    .iter()
                    .map(|array| array.to_vec())
                    .collect::<Vec<_>>(),
            ),
            NbtList::LongArray(long_arrays) => crate::owned::NbtList::LongArray(
                long_arrays
                    .iter()
                    .map(|array| array.to_vec())
                    .collect::<Vec<_>>(),
            ),
        }
    }

    pub fn as_nbt_tags(&self) -> Vec<super::NbtTag> {
        match self {
            NbtList::Empty => vec![],
            NbtList::Byte(bytes) => bytes
                .iter()
                .map(|&byte| super::NbtTag::Byte(byte))
                .collect(),
            NbtList::Short(shorts) => shorts
                .to_vec()
                .into_iter()
                .map(|short| super::NbtTag::Short(short))
                .collect(),
            NbtList::Int(ints) => ints
                .to_vec()
                .into_iter()
                .map(|int| super::NbtTag::Int(int))
                .collect(),
            NbtList::Long(longs) => longs
                .to_vec()
                .into_iter()
                .map(|long| super::NbtTag::Long(long))
                .collect(),
            NbtList::Float(floats) => floats
                .to_vec()
                .into_iter()
                .map(|float| super::NbtTag::Float(float))
                .collect(),
            NbtList::Double(doubles) => doubles
                .to_vec()
                .into_iter()
                .map(|double| super::NbtTag::Double(double))
                .collect(),
            NbtList::ByteArray(byte_arrays) => byte_arrays
                .iter()
                .map(|&array| super::NbtTag::ByteArray(array))
                .collect(),
            NbtList::String(strings) => strings
                .iter()
                .map(|&string| super::NbtTag::String(string))
                .collect(),
            NbtList::List(lists) => lists
                .iter()
                .map(|list| super::NbtTag::List(list.clone()))
                .collect(),
            NbtList::Compound(compounds) => compounds
                .iter()
                .map(|compound| super::NbtTag::Compound(compound.clone()))
                .collect(),
            NbtList::IntArray(int_arrays) => int_arrays
                .iter()
                .map(|array| super::NbtTag::IntArray(array.clone()))
                .collect(),
            NbtList::LongArray(long_arrays) => long_arrays
                .iter()
                .map(|array| super::NbtTag::LongArray(array.clone()))
                .collect(),
        }
    }
}
