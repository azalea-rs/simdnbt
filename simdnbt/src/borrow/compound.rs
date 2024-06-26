use std::{io::Cursor, mem::MaybeUninit};

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_string, unchecked_extend, unchecked_push, unchecked_write_string, write_string,
        END_ID, MAX_DEPTH,
    },
    Error, Mutf8Str,
};

use super::{list::NbtList, tag_alloc::TagAllocator, NbtTag};

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct NbtCompound<'a> {
    values: &'a [(&'a Mutf8Str, NbtTag<'a>)],
}

impl<'a> NbtCompound<'a> {
    /// # Safety
    /// The given TagAllocator must be valid for the lifetime of all the tags in this NBT.
    pub(crate) unsafe fn read(
        data: &mut Cursor<&'a [u8]>,
        alloc: &TagAllocator<'a>,
    ) -> Result<Self, Error> {
        Self::read_with_depth(data, alloc, 0, 0)
    }

    /// # Safety
    /// The given TagAllocator must be valid for the lifetime of all the tags in this NBT.
    pub(crate) unsafe fn read_with_depth(
        data: &mut Cursor<&'a [u8]>,
        alloc: &TagAllocator<'a>,
        compound_depth: usize,
        list_depth: usize,
    ) -> Result<Self, Error> {
        if compound_depth + list_depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }

        let mut tags = alloc.get().named.start(compound_depth);

        let mut tags_buffer = unsafe {
            MaybeUninit::<[MaybeUninit<(&Mutf8Str, NbtTag<'a>)>; 4]>::uninit().assume_init()
        };
        let mut tags_buffer_len: usize = 0;

        loop {
            let tag_type = match data.read_u8() {
                Ok(tag_type) => tag_type,
                Err(_) => {
                    alloc.get().named.finish(tags, compound_depth);
                    return Err(Error::UnexpectedEof);
                }
            };
            if tag_type == END_ID {
                break;
            }

            let tag_name = match read_string(data) {
                Ok(name) => name,
                Err(_) => {
                    alloc.get().named.finish(tags, compound_depth);
                    // the only error read_string can return is UnexpectedEof, so this makes it
                    // slightly faster
                    return Err(Error::UnexpectedEof);
                }
            };
            let tag =
                match NbtTag::read_with_type(data, alloc, tag_type, compound_depth, list_depth) {
                    Ok(tag) => tag,
                    Err(e) => {
                        alloc.get().named.finish(tags, compound_depth);
                        return Err(e);
                    }
                };

            tags_buffer[tags_buffer_len] = MaybeUninit::new((tag_name, tag));
            tags_buffer_len += 1;

            if tags_buffer_len == tags_buffer.len() {
                // writing the tags in groups like this is slightly faster
                for i in 0..tags_buffer_len {
                    tags.push(unsafe { tags_buffer.get_unchecked(i).assume_init_read() });
                }
                tags_buffer_len = 0;
            }
        }

        for i in 0..tags_buffer_len {
            tags.push(unsafe { tags_buffer.get_unchecked(i).assume_init_read() });
        }

        let values = alloc.get().named.finish(tags, compound_depth);

        Ok(Self { values })
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        for (name, tag) in self.values {
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
                    data.extend_from_slice(int_array.as_big_endian());
                }
                NbtTag::LongArray(long_array) => {
                    unsafe {
                        unchecked_extend(data, &long_array.len().to_be_bytes());
                    }
                    data.extend_from_slice(long_array.as_big_endian());
                }
            }
        }
        data.push(END_ID);
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<&NbtTag<'a>> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, value) in self.values {
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
        for (key, _) in self.values {
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
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn keys(&self) -> impl Iterator<Item = &Mutf8Str> {
        self.values.iter().map(|(k, _)| *k)
    }

    pub fn to_owned(&self) -> crate::owned::NbtCompound {
        crate::owned::NbtCompound {
            values: self
                .values
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.to_owned()))
                .collect(),
        }
    }
}
