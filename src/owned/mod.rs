//! The owned variant of NBT. This is useful if you're writing data from scratch or if you can't keep a reference to the original data.

pub mod list;

use std::{io::Cursor, ops::Deref};

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_u32, read_with_u32_length,
        slice_into_u8_big_endian, unchecked_extend, unchecked_push, unchecked_write_string,
        write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID,
        INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    mutf8::Mutf8String,
    Mutf8Str, ReadError,
};

use self::list::ListTag;

/// A complete NBT container. This contains a name and a compound tag.
#[derive(Debug, Clone, PartialEq)]
pub struct Nbt {
    name: Mutf8String,
    tag: CompoundTag,
}
impl Nbt {
    /// Get the name of the NBT compound. This is often an empty string.
    pub fn name(&self) -> &Mutf8Str {
        &self.name
    }
}
impl Deref for Nbt {
    type Target = CompoundTag;

    fn deref(&self) -> &Self::Target {
        &self.tag
    }
}

impl Nbt {
    /// Reads NBT from the given data. Returns `Ok(None)` if there is no data.
    pub fn new(data: &mut Cursor<&[u8]>) -> Result<Option<Nbt>, ReadError> {
        let root_type = data.read_u8().map_err(|_| ReadError::UnexpectedEof)?;
        if root_type == END_ID {
            return Ok(None);
        }
        if root_type != COMPOUND_ID {
            return Err(ReadError::InvalidRootType(root_type));
        }
        let name = read_string(data)?.to_owned();
        let tag = CompoundTag::new(data, 0)?;

        Ok(Some(Nbt { name, tag }))
    }

    /// Writes the NBT to the given buffer.
    pub fn write(&self, data: &mut Vec<u8>) {
        data.push(COMPOUND_ID);
        write_string(data, &self.name);
        self.tag.write(data);
    }
}

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CompoundTag {
    values: Vec<(Mutf8String, Tag)>,
}

impl CompoundTag {
    fn new(data: &mut Cursor<&[u8]>, depth: usize) -> Result<Self, ReadError> {
        if depth > MAX_DEPTH {
            return Err(ReadError::MaxDepthExceeded);
        }
        let mut values = Vec::with_capacity(8);
        loop {
            let tag_type = data.read_u8().map_err(|_| ReadError::UnexpectedEof)?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?.to_owned();

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

/// A single NBT tag.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8) = BYTE_ID,
    Short(i16) = SHORT_ID,
    Int(i32) = INT_ID,
    Long(i64) = LONG_ID,
    Float(f32) = FLOAT_ID,
    Double(f64) = DOUBLE_ID,
    ByteArray(Vec<u8>) = BYTE_ARRAY_ID,
    String(Mutf8String) = STRING_ID,
    List(ListTag) = LIST_ID,
    Compound(CompoundTag) = COMPOUND_ID,
    IntArray(Vec<i32>) = INT_ARRAY_ID,
    LongArray(Vec<i64>) = LONG_ARRAY_ID,
}
impl Tag {
    /// Get the numerical ID of the tag type.
    #[inline]
    pub fn id(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)`
        // `union` between `repr(C)` structs, each of which has the `u8`
        // discriminant as its first field, so we can read the discriminant
        // without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }

    pub fn byte(&self) -> Option<i8> {
        match self {
            Tag::Byte(byte) => Some(*byte),
            _ => None,
        }
    }
    pub fn byte_mut(&mut self) -> Option<&mut i8> {
        match self {
            Tag::Byte(byte) => Some(byte),
            _ => None,
        }
    }
    pub fn short(&self) -> Option<i16> {
        match self {
            Tag::Short(short) => Some(*short),
            _ => None,
        }
    }
    pub fn short_mut(&mut self) -> Option<&mut i16> {
        match self {
            Tag::Short(short) => Some(short),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i32> {
        match self {
            Tag::Int(int) => Some(*int),
            _ => None,
        }
    }
    pub fn int_mut(&mut self) -> Option<&mut i32> {
        match self {
            Tag::Int(int) => Some(int),
            _ => None,
        }
    }
    pub fn long(&self) -> Option<i64> {
        match self {
            Tag::Long(long) => Some(*long),
            _ => None,
        }
    }
    pub fn long_mut(&mut self) -> Option<&mut i64> {
        match self {
            Tag::Long(long) => Some(long),
            _ => None,
        }
    }
    pub fn float(&self) -> Option<f32> {
        match self {
            Tag::Float(float) => Some(*float),
            _ => None,
        }
    }
    pub fn float_mut(&mut self) -> Option<&mut f32> {
        match self {
            Tag::Float(float) => Some(float),
            _ => None,
        }
    }
    pub fn double(&self) -> Option<f64> {
        match self {
            Tag::Double(double) => Some(*double),
            _ => None,
        }
    }
    pub fn double_mut(&mut self) -> Option<&mut f64> {
        match self {
            Tag::Double(double) => Some(double),
            _ => None,
        }
    }
    pub fn byte_array(&self) -> Option<&[u8]> {
        match self {
            Tag::ByteArray(byte_array) => Some(byte_array),
            _ => None,
        }
    }
    pub fn byte_array_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            Tag::ByteArray(byte_array) => Some(byte_array),
            _ => None,
        }
    }
    pub fn string(&self) -> Option<&Mutf8Str> {
        match self {
            Tag::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn string_mut(&mut self) -> Option<&mut Mutf8String> {
        match self {
            Tag::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn list(&self) -> Option<&ListTag> {
        match self {
            Tag::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn list_mut(&mut self) -> Option<&mut ListTag> {
        match self {
            Tag::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn compound(&self) -> Option<&CompoundTag> {
        match self {
            Tag::Compound(compound) => Some(compound),
            _ => None,
        }
    }
    pub fn compound_mut(&mut self) -> Option<&mut CompoundTag> {
        match self {
            Tag::Compound(compound) => Some(compound),
            _ => None,
        }
    }
    pub fn int_array(&self) -> Option<&[i32]> {
        match self {
            Tag::IntArray(int_array) => Some(int_array),
            _ => None,
        }
    }
    pub fn int_array_mut(&mut self) -> Option<&mut Vec<i32>> {
        match self {
            Tag::IntArray(int_array) => Some(int_array),
            _ => None,
        }
    }
    pub fn long_array(&self) -> Option<&[i64]> {
        match self {
            Tag::LongArray(long_array) => Some(long_array),
            _ => None,
        }
    }
    pub fn long_array_mut(&mut self) -> Option<&mut Vec<i64>> {
        match self {
            Tag::LongArray(long_array) => Some(long_array),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use byteorder::WriteBytesExt;
    use flate2::read::GzDecoder;

    use crate::common::{INT_ID, LIST_ID, LONG_ID};

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = Nbt::new(&mut Cursor::new(include_bytes!(
            "../../tests/hello_world.nbt"
        )))
        .unwrap()
        .unwrap();

        assert_eq!(
            nbt.string("name"),
            Some(Mutf8Str::from_str("Bananrama").as_ref())
        );
        assert_eq!(nbt.name().to_str(), "hello world");
    }

    #[test]
    fn simple_player() {
        let src = include_bytes!("../../tests/simple_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::new(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        assert_eq!(nbt.int("PersistentId"), Some(1946940766));
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::new(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn read_write_complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::new(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        let mut out = Vec::new();
        nbt.write(&mut out);
        let nbt = Nbt::new(&mut Cursor::new(&out)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn inttest_1023() {
        let nbt = Nbt::new(&mut Cursor::new(include_bytes!(
            "../../tests/inttest1023.nbt"
        )))
        .unwrap()
        .unwrap();

        let ints = nbt.list("").unwrap().ints().unwrap();

        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i32, item);
        }
        assert_eq!(ints.len(), 1023);
    }

    #[test]
    fn inttest_1024() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(INT_ID).unwrap();
        data.write_i32::<BE>(1024).unwrap();
        for i in 0..1024 {
            data.write_i32::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        let nbt = Nbt::new(&mut Cursor::new(&data)).unwrap().unwrap();
        let ints = nbt.list("").unwrap().ints().unwrap();
        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i32, item);
        }
        assert_eq!(ints.len(), 1024);
    }

    #[test]
    fn inttest_1021() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(INT_ID).unwrap();
        data.write_i32::<BE>(1021).unwrap();
        for i in 0..1021 {
            data.write_i32::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        let nbt = Nbt::new(&mut Cursor::new(&data)).unwrap().unwrap();
        let ints = nbt.list("").unwrap().ints().unwrap();
        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i32, item);
        }
        assert_eq!(ints.len(), 1021);
    }

    #[test]
    fn longtest_1023() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LONG_ID).unwrap();
        data.write_i32::<BE>(1023).unwrap();
        for i in 0..1023 {
            data.write_i64::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        let nbt = Nbt::new(&mut Cursor::new(&data)).unwrap().unwrap();
        let ints = nbt.list("").unwrap().longs().unwrap();
        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i64, item);
        }
        assert_eq!(ints.len(), 1023);
    }
}
