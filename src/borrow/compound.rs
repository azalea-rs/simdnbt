use std::io::Cursor;

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_with_u32_length, unchecked_extend,
        unchecked_push, unchecked_write_string, write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID,
        DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID,
        MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    Mutf8Str, ReadError,
};

use super::{list::ListTag, Tag};

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default, PartialEq)]
pub struct CompoundTag<'a> {
    values: Vec<(&'a Mutf8Str, Tag<'a>)>,
}

impl<'a> CompoundTag<'a> {
    pub fn new(data: &mut Cursor<&'a [u8]>, depth: usize) -> Result<Self, ReadError> {
        if depth > MAX_DEPTH {
            return Err(ReadError::MaxDepthExceeded);
        }
        let mut values = Vec::with_capacity(4);
        loop {
            let tag_type = data.read_u8().map_err(|_| ReadError::UnexpectedEof)?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?;

            match tag_type {
                BYTE_ID => values.push((
                    tag_name,
                    Tag::Byte(data.read_i8().map_err(|_| ReadError::UnexpectedEof)?),
                )),
                SHORT_ID => values.push((
                    tag_name,
                    Tag::Short(
                        data.read_i16::<BE>()
                            .map_err(|_| ReadError::UnexpectedEof)?,
                    ),
                )),
                INT_ID => values.push((
                    tag_name,
                    Tag::Int(
                        data.read_i32::<BE>()
                            .map_err(|_| ReadError::UnexpectedEof)?,
                    ),
                )),
                LONG_ID => values.push((
                    tag_name,
                    Tag::Long(
                        data.read_i64::<BE>()
                            .map_err(|_| ReadError::UnexpectedEof)?,
                    ),
                )),
                FLOAT_ID => values.push((
                    tag_name,
                    Tag::Float(
                        data.read_f32::<BE>()
                            .map_err(|_| ReadError::UnexpectedEof)?,
                    ),
                )),
                DOUBLE_ID => values.push((
                    tag_name,
                    Tag::Double(
                        data.read_f64::<BE>()
                            .map_err(|_| ReadError::UnexpectedEof)?,
                    ),
                )),
                BYTE_ARRAY_ID => {
                    values.push((tag_name, Tag::ByteArray(read_with_u32_length(data, 1)?)))
                }
                STRING_ID => values.push((tag_name, Tag::String(read_string(data)?))),
                LIST_ID => values.push((tag_name, Tag::List(ListTag::new(data, depth + 1)?))),
                COMPOUND_ID => {
                    values.push((tag_name, Tag::Compound(CompoundTag::new(data, depth + 1)?)))
                }
                INT_ARRAY_ID => values.push((tag_name, Tag::IntArray(read_int_array(data)?))),
                LONG_ARRAY_ID => values.push((tag_name, Tag::LongArray(read_long_array(data)?))),
                _ => return Err(ReadError::UnknownTagId(tag_type)),
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
                Tag::Byte(byte) => unsafe {
                    unchecked_push(data, *byte as u8);
                },
                Tag::Short(short) => unsafe {
                    unchecked_extend(data, &short.to_be_bytes());
                },
                Tag::Int(int) => unsafe {
                    unchecked_extend(data, &int.to_be_bytes());
                },
                Tag::Long(long) => {
                    data.extend_from_slice(&long.to_be_bytes());
                }
                Tag::Float(float) => unsafe {
                    unchecked_extend(data, &float.to_be_bytes());
                },
                Tag::Double(double) => {
                    data.extend_from_slice(&double.to_be_bytes());
                }
                Tag::ByteArray(byte_array) => {
                    unsafe {
                        unchecked_extend(data, &byte_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(byte_array);
                }
                Tag::String(string) => {
                    write_string(data, string);
                }
                Tag::List(list) => {
                    list.write(data);
                }
                Tag::Compound(compound) => {
                    compound.write(data);
                }
                Tag::IntArray(int_array) => {
                    unsafe {
                        unchecked_extend(data, &int_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(&int_array.as_big_endian());
                }
                Tag::LongArray(long_array) => {
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
    pub fn get(&self, name: &str) -> Option<&Tag<'a>> {
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
    pub fn list(&self, name: &str) -> Option<&ListTag<'a>> {
        self.get(name).and_then(|tag| tag.list())
    }
    pub fn compound(&self, name: &str) -> Option<&CompoundTag<'a>> {
        self.get(name).and_then(|tag| tag.compound())
    }
    pub fn int_array(&self, name: &str) -> Option<Vec<i32>> {
        self.get(name).and_then(|tag| tag.int_array())
    }
    pub fn long_array(&self, name: &str) -> Option<Vec<i64>> {
        self.get(name).and_then(|tag| tag.long_array())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &Tag<'a>)> {
        self.values.iter().map(|(k, v)| (*k, v))
    }
}
