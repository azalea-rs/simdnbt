use std::io::Cursor;

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_with_u32_length, unchecked_extend,
        unchecked_push, unchecked_write_string, write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID,
        DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID,
        MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    Error, Mutf8Str,
};

use super::{list::NbtList, NbtTag};

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default, PartialEq)]
pub struct NbtCompound<'a> {
    values: Vec<(&'a Mutf8Str, NbtTag<'a>)>,
}

impl<'a> NbtCompound<'a> {
    pub fn read(data: &mut Cursor<&'a [u8]>) -> Result<Self, Error> {
        Self::read_with_depth(data, 0)
    }

    pub fn read_with_depth(data: &mut Cursor<&'a [u8]>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let mut values = Vec::with_capacity(4);
        loop {
            let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?;

            match tag_type {
                BYTE_ID => values.push((
                    tag_name,
                    NbtTag::Byte(data.read_i8().map_err(|_| Error::UnexpectedEof)?),
                )),
                SHORT_ID => values.push((
                    tag_name,
                    NbtTag::Short(data.read_i16::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                INT_ID => values.push((
                    tag_name,
                    NbtTag::Int(data.read_i32::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                LONG_ID => values.push((
                    tag_name,
                    NbtTag::Long(data.read_i64::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                FLOAT_ID => values.push((
                    tag_name,
                    NbtTag::Float(data.read_f32::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                DOUBLE_ID => values.push((
                    tag_name,
                    NbtTag::Double(data.read_f64::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                BYTE_ARRAY_ID => {
                    values.push((tag_name, NbtTag::ByteArray(read_with_u32_length(data, 1)?)))
                }
                STRING_ID => values.push((tag_name, NbtTag::String(read_string(data)?))),
                LIST_ID => values.push((tag_name, NbtTag::List(NbtList::read(data, depth + 1)?))),
                COMPOUND_ID => values.push((
                    tag_name,
                    NbtTag::Compound(NbtCompound::read_with_depth(data, depth + 1)?),
                )),
                INT_ARRAY_ID => values.push((tag_name, NbtTag::IntArray(read_int_array(data)?))),
                LONG_ARRAY_ID => values.push((tag_name, NbtTag::LongArray(read_long_array(data)?))),
                _ => return Err(Error::UnknownTagId(tag_type)),
            }
        }
        Ok(Self { values })
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        for (name, tag) in &self.values {
            // reserve 4 bytes extra so we can avoid reallocating for small tags
            data.reserve(1 + 2 + name.len() + 4);
            // SAFETY: We just reserved enough space for the tag ID, the name length, the name, and
            // 4 bytes of tag data.
            unsafe {
                unchecked_push(data, tag.id());
                unchecked_write_string(data, name);
            }
            match tag {
                NbtTag::Byte(byte) => unsafe {
                    unchecked_push(data, *byte as u8);
                },
                NbtTag::Short(short) => unsafe {
                    unchecked_extend(data, &short.to_be_bytes());
                },
                NbtTag::Int(int) => unsafe {
                    unchecked_extend(data, &int.to_be_bytes());
                },
                NbtTag::Long(long) => {
                    data.extend_from_slice(&long.to_be_bytes());
                }
                NbtTag::Float(float) => unsafe {
                    unchecked_extend(data, &float.to_be_bytes());
                },
                NbtTag::Double(double) => {
                    data.extend_from_slice(&double.to_be_bytes());
                }
                NbtTag::ByteArray(byte_array) => {
                    unsafe {
                        unchecked_extend(data, &byte_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(byte_array);
                }
                NbtTag::String(string) => {
                    write_string(data, string);
                }
                NbtTag::List(list) => {
                    list.write(data);
                }
                NbtTag::Compound(compound) => {
                    compound.write(data);
                }
                NbtTag::IntArray(int_array) => {
                    unsafe {
                        unchecked_extend(data, &int_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(&int_array.as_big_endian());
                }
                NbtTag::LongArray(long_array) => {
                    unsafe {
                        unchecked_extend(data, &long_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(&long_array.as_big_endian());
                }
            }
        }
        data.push(END_ID);
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<&NbtTag<'a>> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, value) in &self.values {
            if key == &name {
                return Some(value);
            }
        }
        None
    }

    /// Returns whether there is a tag with the given name.
    pub fn contains(&self, name: &str) -> bool {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, _) in &self.values {
            if key == &name {
                return true;
            }
        }
        false
    }

    pub fn byte(&self, name: &str) -> Option<i8> {
        self.get(name).and_then(|tag| tag.byte())
    }
    pub fn short(&self, name: &str) -> Option<i16> {
        self.get(name).and_then(|tag| tag.short())
    }
    pub fn int(&self, name: &str) -> Option<i32> {
        self.get(name).and_then(|tag| tag.int())
    }
    pub fn long(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|tag| tag.long())
    }
    pub fn float(&self, name: &str) -> Option<f32> {
        self.get(name).and_then(|tag| tag.float())
    }
    pub fn double(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(|tag| tag.double())
    }
    pub fn byte_array(&self, name: &str) -> Option<&[u8]> {
        self.get(name).and_then(|tag| tag.byte_array())
    }
    pub fn string(&self, name: &str) -> Option<&Mutf8Str> {
        self.get(name).and_then(|tag| tag.string())
    }
    pub fn list(&self, name: &str) -> Option<&NbtList<'a>> {
        self.get(name).and_then(|tag| tag.list())
    }
    pub fn compound(&self, name: &str) -> Option<&NbtCompound<'a>> {
        self.get(name).and_then(|tag| tag.compound())
    }
    pub fn int_array(&self, name: &str) -> Option<Vec<i32>> {
        self.get(name).and_then(|tag| tag.int_array())
    }
    pub fn long_array(&self, name: &str) -> Option<Vec<i64>> {
        self.get(name).and_then(|tag| tag.long_array())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &NbtTag<'a>)> {
        self.values.iter().map(|(k, v)| (*k, v))
    }
}
