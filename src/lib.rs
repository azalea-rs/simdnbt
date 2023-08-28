//! an unnecessarily fast nbt decoder.
//!
//! afaik, this is currently the fastest nbt decoder in existence.
//!
//! ```
//! use simdnbt::Nbt;
//! use std::io::Cursor;
//!
//! let nbt = Nbt::new(&mut Cursor::new(include_bytes!("../tests/hello_world.nbt"))).unwrap().unwrap();
//! assert_eq!(nbt.name().to_str(), "hello world");
//! assert_eq!(nbt.string("name").unwrap().to_str(), "Bananrama");
//! ```

mod error;
mod mutf8;

use std::{io::Cursor, ops::Deref, slice};

use byteorder::{ReadBytesExt, BE};
pub use error::Error;
pub use mutf8::Mutf8Str;

/// A complete NBT container. This contains a name and a compound tag.
#[derive(Debug)]
pub struct Nbt<'a> {
    name: &'a Mutf8Str,
    tag: CompoundTag<'a>,
}
impl<'a> Nbt<'a> {
    /// Get the name of the NBT compound. This is often an empty string.
    pub fn name(&self) -> &'a Mutf8Str {
        self.name
    }
}
impl<'a> Deref for Nbt<'a> {
    type Target = CompoundTag<'a>;

    fn deref(&self) -> &Self::Target {
        &self.tag
    }
}

fn read_with_u16_length<'a>(data: &mut Cursor<&'a [u8]>, width: usize) -> Result<&'a [u8], Error> {
    let length = data.read_u16::<BE>()?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(Error::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

fn read_with_u32_length<'a>(data: &mut Cursor<&'a [u8]>, width: usize) -> Result<&'a [u8], Error> {
    let length = data.read_u32::<BE>()?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(Error::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

fn read_string<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a Mutf8Str, Error> {
    let data = read_with_u16_length(data, 1)?;
    Ok(Mutf8Str::from_slice(data))
}

impl<'a> Nbt<'a> {
    /// Reads NBT from the given data. Returns `Ok(None)` if there is no data.
    pub fn new(data: &mut Cursor<&'a [u8]>) -> Result<Option<Nbt<'a>>, Error> {
        let root_type = data.read_u8()?;
        if root_type == END_ID {
            return Ok(None);
        }
        if root_type != COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type));
        }
        let name = read_string(data)?;
        let tag = CompoundTag::new(data, 0)?;

        Ok(Some(Nbt { name, tag }))
    }
}

const END_ID: u8 = 0;
const BYTE_ID: u8 = 1;
const SHORT_ID: u8 = 2;
const INT_ID: u8 = 3;
const LONG_ID: u8 = 4;
const FLOAT_ID: u8 = 5;
const DOUBLE_ID: u8 = 6;
const BYTE_ARRAY_ID: u8 = 7;
const STRING_ID: u8 = 8;
const LIST_ID: u8 = 9;
const COMPOUND_ID: u8 = 10;
const INT_ARRAY_ID: u8 = 11;
const LONG_ARRAY_ID: u8 = 12;

const MAX_DEPTH: usize = 512;

/// A list of named tags. The order of the tags is preserved.
#[derive(Debug, Default)]
pub struct CompoundTag<'a> {
    values: Vec<(&'a Mutf8Str, Tag<'a>)>,
}

impl<'a> CompoundTag<'a> {
    fn new(data: &mut Cursor<&'a [u8]>, depth: usize) -> Result<Self, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let mut values = Vec::with_capacity(4);
        loop {
            let tag_type = data.read_u8()?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?;

            match tag_type {
                BYTE_ID => values.push((tag_name, Tag::Byte(data.read_i8()?))),
                SHORT_ID => values.push((tag_name, Tag::Short(data.read_i16::<BE>()?))),
                INT_ID => values.push((tag_name, Tag::Int(data.read_i32::<BE>()?))),
                LONG_ID => values.push((tag_name, Tag::Long(data.read_i64::<BE>()?))),
                FLOAT_ID => values.push((tag_name, Tag::Float(data.read_f32::<BE>()?))),
                DOUBLE_ID => values.push((tag_name, Tag::Double(data.read_f64::<BE>()?))),
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
                _ => return Err(Error::UnknownTagId(tag_type)),
            }
        }
        Ok(Self { values })
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
        match self.get(name) {
            Some(Tag::Byte(byte)) => Some(*byte),
            _ => None,
        }
    }
    pub fn short(&self, name: &str) -> Option<i16> {
        match self.get(name) {
            Some(Tag::Short(short)) => Some(*short),
            _ => None,
        }
    }
    pub fn int(&self, name: &str) -> Option<i32> {
        match self.get(name) {
            Some(Tag::Int(int)) => Some(*int),
            _ => None,
        }
    }
    pub fn long(&self, name: &str) -> Option<i64> {
        match self.get(name) {
            Some(Tag::Long(long)) => Some(*long),
            _ => None,
        }
    }
    pub fn float(&self, name: &str) -> Option<f32> {
        match self.get(name) {
            Some(Tag::Float(float)) => Some(*float),
            _ => None,
        }
    }
    pub fn double(&self, name: &str) -> Option<&f64> {
        match self.get(name) {
            Some(Tag::Double(double)) => Some(double),
            _ => None,
        }
    }
    pub fn byte_array(&self, name: &str) -> Option<&[u8]> {
        match self.get(name) {
            Some(Tag::ByteArray(byte_array)) => Some(byte_array),
            _ => None,
        }
    }
    pub fn string(&self, name: &str) -> Option<&Mutf8Str> {
        match self.get(name) {
            Some(Tag::String(string)) => Some(string),
            _ => None,
        }
    }
    pub fn list(&self, name: &str) -> Option<&ListTag<'a>> {
        match self.get(name) {
            Some(Tag::List(list)) => Some(list),
            _ => None,
        }
    }
    pub fn compound(&self, name: &str) -> Option<&CompoundTag<'a>> {
        match self.get(name) {
            Some(Tag::Compound(compound)) => Some(compound),
            _ => None,
        }
    }
    pub fn int_array(&self, name: &str) -> Option<&[i32]> {
        match self.get(name) {
            Some(Tag::IntArray(int_array)) => Some(int_array),
            _ => None,
        }
    }
    pub fn long_array(&self, name: &str) -> Option<&[i64]> {
        match self.get(name) {
            Some(Tag::LongArray(long_array)) => Some(long_array),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &Tag<'a>)> {
        self.values.iter().map(|(k, v)| (*k, v))
    }
}

fn read_u8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    read_with_u32_length(data, 1)
}
fn read_i8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [i8], Error> {
    Ok(slice_u8_into_i8(read_u8_array(data)?))
}
fn read_short_array(data: &mut Cursor<&[u8]>) -> Result<Vec<i16>, Error> {
    let array_bytes = read_with_u32_length(data, 2)?;
    let mut array_bytes_cursor = Cursor::new(array_bytes);
    let length = array_bytes.len() / 2;
    let mut shorts = Vec::with_capacity(length);
    for _ in 0..length {
        shorts.push(array_bytes_cursor.read_i16::<BE>()?);
    }
    Ok(shorts)
}
fn read_int_array(data: &mut Cursor<&[u8]>) -> Result<Vec<i32>, Error> {
    let array_bytes = read_with_u32_length(data, 4)?;
    let mut array_bytes_cursor = Cursor::new(array_bytes);
    let length = array_bytes.len() / 4;
    let mut ints = Vec::with_capacity(length);
    for _ in 0..length {
        ints.push(array_bytes_cursor.read_i32::<BE>()?);
    }
    Ok(ints)
}
fn read_long_array(data: &mut Cursor<&[u8]>) -> Result<Vec<i64>, Error> {
    let array_bytes = read_with_u32_length(data, 8)?;
    let mut array_bytes_cursor = Cursor::new(array_bytes);
    let length = array_bytes.len() / 8;
    let mut longs = Vec::with_capacity(length);
    for _ in 0..length {
        longs.push(array_bytes_cursor.read_i64::<BE>()?);
    }
    Ok(longs)
}
fn read_float_array(data: &mut Cursor<&[u8]>) -> Result<Vec<f32>, Error> {
    let array_bytes = read_with_u32_length(data, 4)?;
    let mut array_bytes_cursor = Cursor::new(array_bytes);
    let length = array_bytes.len() / 4;
    let mut floats = Vec::with_capacity(length);
    for _ in 0..length {
        floats.push(array_bytes_cursor.read_f32::<BE>()?);
    }
    Ok(floats)
}
fn read_double_array(data: &mut Cursor<&[u8]>) -> Result<Vec<f64>, Error> {
    let array_bytes = read_with_u32_length(data, 8)?;
    let mut array_bytes_cursor = Cursor::new(array_bytes);
    let length = array_bytes.len() / 8;
    let mut doubles = Vec::with_capacity(length);
    for _ in 0..length {
        doubles.push(array_bytes_cursor.read_f64::<BE>()?);
    }
    Ok(doubles)
}
fn slice_u8_into_i8(s: &[u8]) -> &[i8] {
    unsafe { slice::from_raw_parts(s.as_ptr() as *const i8, s.len()) }
}

/// A single NBT tag.
#[derive(Debug)]
pub enum Tag<'a> {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(&'a [u8]),
    String(&'a Mutf8Str),
    List(ListTag<'a>),
    Compound(CompoundTag<'a>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}
impl<'a> Tag<'a> {
    pub fn byte(&self) -> Option<i8> {
        match self {
            Tag::Byte(byte) => Some(*byte),
            _ => None,
        }
    }
    pub fn short(&self) -> Option<i16> {
        match self {
            Tag::Short(short) => Some(*short),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i32> {
        match self {
            Tag::Int(int) => Some(*int),
            _ => None,
        }
    }
    pub fn long(&self) -> Option<i64> {
        match self {
            Tag::Long(long) => Some(*long),
            _ => None,
        }
    }
    pub fn float(&self) -> Option<f32> {
        match self {
            Tag::Float(float) => Some(*float),
            _ => None,
        }
    }
    pub fn double(&self) -> Option<f64> {
        match self {
            Tag::Double(double) => Some(*double),
            _ => None,
        }
    }
    pub fn byte_array(&self) -> Option<&[u8]> {
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
    pub fn list(&self) -> Option<&ListTag<'a>> {
        match self {
            Tag::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn compound(&self) -> Option<&CompoundTag<'a>> {
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
    pub fn long_array(&self) -> Option<&[i64]> {
        match self {
            Tag::LongArray(long_array) => Some(long_array),
            _ => None,
        }
    }
}

/// A list of NBT tags of a single type.
#[derive(Debug, Default)]
pub enum ListTag<'a> {
    #[default]
    Empty,
    Byte(&'a [i8]),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
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
        let tag_type = data.read_u8()?;
        Ok(match tag_type {
            END_ID => {
                let _length = data.read_u32::<BE>()?;
                ListTag::Empty
            }
            BYTE_ID => ListTag::Byte(read_i8_array(data)?),
            SHORT_ID => ListTag::Short(read_short_array(data)?),
            INT_ID => ListTag::Int(read_int_array(data)?),
            LONG_ID => ListTag::Long(read_long_array(data)?),
            FLOAT_ID => ListTag::Float(read_float_array(data)?),
            DOUBLE_ID => ListTag::Double(read_double_array(data)?),
            BYTE_ARRAY_ID => ListTag::ByteArray(read_u8_array(data)?),
            STRING_ID => ListTag::String({
                let length = data.read_u32::<BE>()?;
                // arbitrary number to prevent big allocations
                let mut strings = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    strings.push(read_string(data)?)
                }
                strings
            }),
            LIST_ID => ListTag::List({
                let length = data.read_u32::<BE>()?;
                // arbitrary number to prevent big allocations
                let mut lists = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    lists.push(ListTag::new(data, depth + 1)?)
                }
                lists
            }),
            COMPOUND_ID => ListTag::Compound({
                let length = data.read_u32::<BE>()?;
                // arbitrary number to prevent big allocations
                let mut compounds = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    compounds.push(CompoundTag::new(data, depth + 1)?)
                }
                compounds
            }),
            INT_ARRAY_ID => ListTag::IntArray({
                let length = data.read_u32::<BE>()?;
                // arbitrary number to prevent big allocations
                let mut arrays = Vec::with_capacity(length.min(128) as usize);
                for _ in 0..length {
                    arrays.push(read_int_array(data)?)
                }
                arrays
            }),
            LONG_ARRAY_ID => ListTag::LongArray({
                let length = data.read_u32::<BE>()?;
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
    pub fn shorts(&self) -> Option<&[i16]> {
        match self {
            ListTag::Short(shorts) => Some(shorts),
            _ => None,
        }
    }
    pub fn ints(&self) -> Option<&[i32]> {
        match self {
            ListTag::Int(ints) => Some(ints),
            _ => None,
        }
    }
    pub fn longs(&self) -> Option<&[i64]> {
        match self {
            ListTag::Long(longs) => Some(longs),
            _ => None,
        }
    }
    pub fn floats(&self) -> Option<&[f32]> {
        match self {
            ListTag::Float(floats) => Some(floats),
            _ => None,
        }
    }
    pub fn doubles(&self) -> Option<&[f64]> {
        match self {
            ListTag::Double(doubles) => Some(doubles),
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

#[cfg(test)]
mod tests {
    use std::io::Read;

    use flate2::read::GzDecoder;

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = Nbt::new(&mut Cursor::new(include_bytes!("../tests/hello_world.nbt")))
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
        let src = include_bytes!("../tests/simple_player.dat").to_vec();
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
        let src = include_bytes!("../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::new(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }
}
