use std::io::Cursor;

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_with_u32_length,
        slice_into_u8_big_endian, unchecked_extend, unchecked_push, unchecked_write_string,
        write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID,
        INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    mutf8::Mutf8String,
    Error, Mutf8Str,
};

use super::{list::ListTag, Tag};

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CompoundTag {
    values: Vec<(Mutf8String, Tag)>,
}

impl CompoundTag {
    pub fn new(data: &mut Cursor<&[u8]>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let mut values = Vec::with_capacity(8);
        loop {
            let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?.to_owned();

            match tag_type {
                BYTE_ID => values.push((
                    tag_name,
                    Tag::Byte(data.read_i8().map_err(|_| Error::UnexpectedEof)?),
                )),
                SHORT_ID => values.push((
                    tag_name,
                    Tag::Short(data.read_i16::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                INT_ID => values.push((
                    tag_name,
                    Tag::Int(data.read_i32::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                LONG_ID => values.push((
                    tag_name,
                    Tag::Long(data.read_i64::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                FLOAT_ID => values.push((
                    tag_name,
                    Tag::Float(data.read_f32::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                DOUBLE_ID => values.push((
                    tag_name,
                    Tag::Double(data.read_f64::<BE>().map_err(|_| Error::UnexpectedEof)?),
                )),
                BYTE_ARRAY_ID => values.push((
                    tag_name,
                    Tag::ByteArray(read_with_u32_length(data, 1)?.to_owned()),
                )),
                STRING_ID => values.push((tag_name, Tag::String(read_string(data)?.to_owned()))),
                LIST_ID => values.push((tag_name, Tag::List(ListTag::new(data, depth + 1)?))),
                COMPOUND_ID => {
                    values.push((tag_name, Tag::Compound(CompoundTag::new(data, depth + 1)?)))
                }
                INT_ARRAY_ID => {
                    values.push((tag_name, Tag::IntArray(read_int_array(data)?.to_vec())))
                }
                LONG_ARRAY_ID => {
                    values.push((tag_name, Tag::LongArray(read_long_array(data)?.to_vec())))
                }
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
                    data.extend_from_slice(&slice_into_u8_big_endian(int_array));
                }
                Tag::LongArray(long_array) => {
                    unsafe {
                        unchecked_extend(data, &long_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(&slice_into_u8_big_endian(long_array));
                }
            }
        }
        data.push(END_ID);
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<&Tag> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, value) in &self.values {
            if key.as_str() == name {
                return Some(value);
            }
        }
        None
    }

    #[inline]
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Tag> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, value) in &mut self.values {
            if key.as_str() == name {
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
            if key.as_str() == name {
                return true;
            }
        }
        false
    }

    pub fn byte(&self, name: &str) -> Option<i8> {
        self.get(name).and_then(|tag| tag.byte())
    }
    pub fn byte_mut(&mut self, name: &str) -> Option<&mut i8> {
        self.get_mut(name).and_then(|tag| tag.byte_mut())
    }
    pub fn short(&self, name: &str) -> Option<i16> {
        self.get(name).and_then(|tag| tag.short())
    }
    pub fn short_mut(&mut self, name: &str) -> Option<&mut i16> {
        self.get_mut(name).and_then(|tag| tag.short_mut())
    }
    pub fn int(&self, name: &str) -> Option<i32> {
        self.get(name).and_then(|tag| tag.int())
    }
    pub fn int_mut(&mut self, name: &str) -> Option<&mut i32> {
        self.get_mut(name).and_then(|tag| tag.int_mut())
    }
    pub fn long(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|tag| tag.long())
    }
    pub fn long_mut(&mut self, name: &str) -> Option<&mut i64> {
        self.get_mut(name).and_then(|tag| tag.long_mut())
    }
    pub fn float(&self, name: &str) -> Option<f32> {
        self.get(name).and_then(|tag| tag.float())
    }
    pub fn float_mut(&mut self, name: &str) -> Option<&mut f32> {
        self.get_mut(name).and_then(|tag| tag.float_mut())
    }
    pub fn double(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(|tag| tag.double())
    }
    pub fn double_mut(&mut self, name: &str) -> Option<&mut f64> {
        self.get_mut(name).and_then(|tag| tag.double_mut())
    }
    pub fn byte_array(&self, name: &str) -> Option<&[u8]> {
        self.get(name).and_then(|tag| tag.byte_array())
    }
    pub fn byte_array_mut(&mut self, name: &str) -> Option<&mut Vec<u8>> {
        self.get_mut(name).and_then(|tag| tag.byte_array_mut())
    }
    pub fn string(&self, name: &str) -> Option<&Mutf8Str> {
        self.get(name).and_then(|tag| tag.string())
    }
    pub fn string_mut(&mut self, name: &str) -> Option<&mut Mutf8String> {
        self.get_mut(name).and_then(|tag| tag.string_mut())
    }
    pub fn list(&self, name: &str) -> Option<&ListTag> {
        self.get(name).and_then(|tag| tag.list())
    }
    pub fn list_mut(&mut self, name: &str) -> Option<&mut ListTag> {
        self.get_mut(name).and_then(|tag| tag.list_mut())
    }
    pub fn compound(&self, name: &str) -> Option<&CompoundTag> {
        self.get(name).and_then(|tag| tag.compound())
    }
    pub fn compound_mut(&mut self, name: &str) -> Option<&mut CompoundTag> {
        self.get_mut(name).and_then(|tag| tag.compound_mut())
    }
    pub fn int_array(&self, name: &str) -> Option<&[i32]> {
        self.get(name).and_then(|tag| tag.int_array())
    }
    pub fn int_array_mut(&mut self, name: &str) -> Option<&mut Vec<i32>> {
        self.get_mut(name).and_then(|tag| tag.int_array_mut())
    }
    pub fn long_array(&self, name: &str) -> Option<&[i64]> {
        self.get(name).and_then(|tag| tag.long_array())
    }
    pub fn long_array_mut(&mut self, name: &str) -> Option<&mut Vec<i64>> {
        self.get_mut(name).and_then(|tag| tag.long_array_mut())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &Tag)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Mutf8Str, &mut Tag)> {
        self.values.iter_mut().map(|(k, v)| (k.as_str(), v))
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn keys(&self) -> impl Iterator<Item = &Mutf8Str> {
        self.values.iter().map(|(k, _)| k.as_str())
    }
    pub fn keys_mut(&mut self) -> impl Iterator<Item = &mut Mutf8String> {
        self.values.iter_mut().map(|(k, _)| k)
    }
    pub fn values(&self) -> impl Iterator<Item = &Tag> {
        self.values.iter().map(|(_, v)| v)
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Tag> {
        self.values.iter_mut().map(|(_, v)| v)
    }
    pub fn into_iter(self) -> impl Iterator<Item = (Mutf8String, Tag)> {
        self.values.into_iter()
    }
    pub fn clear(&mut self) {
        self.values.clear();
    }
    pub fn insert(&mut self, name: Mutf8String, tag: Tag) {
        self.values.push((name, tag));
    }
    pub fn remove(&mut self, name: &str) -> Option<Tag> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for i in 0..self.values.len() {
            if self.values[i].0.as_str() == name {
                return Some(self.values.remove(i).1);
            }
        }
        None
    }
}
