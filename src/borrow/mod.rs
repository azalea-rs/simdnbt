//! The borrowed variant of NBT. This is useful if you're only reading data and you can keep a reference to the original buffer.

pub mod list;

use std::{io::Cursor, ops::Deref};

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_u32, read_with_u32_length,
        BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID,
        LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    Error, Mutf8Str,
};

use self::list::ListTag;

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

impl<'a> Nbt<'a> {
    /// Reads NBT from the given data. Returns `Ok(None)` if there is no data.
    pub fn new(data: &mut Cursor<&'a [u8]>) -> Result<Option<Nbt<'a>>, Error> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
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
            let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
            if tag_type == END_ID {
                break;
            }
            let tag_name = read_string(data)?;

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
    pub fn int_array(&self, name: &str) -> Option<&[i32]> {
        self.get(name).and_then(|tag| tag.int_array())
    }
    pub fn long_array(&self, name: &str) -> Option<&[i64]> {
        self.get(name).and_then(|tag| tag.long_array())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &Tag<'a>)> {
        self.values.iter().map(|(k, v)| (*k, v))
    }
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

    // #[test]
    // fn generate_inttest() {
    //     use byteorder::WriteBytesExt;

    //     let mut out = Vec::new();
    //     out.write_u8(COMPOUND_ID).unwrap();
    //     out.write_u16::<BE>(0).unwrap();
    //     out.write_u8(LIST_ID).unwrap();
    //     out.write_u16::<BE>(0).unwrap();
    //     out.write_u8(INT_ID).unwrap();
    //     out.write_i32::<BE>(1023).unwrap();
    //     for i in 0..1023 {
    //         out.write_i32::<BE>(i).unwrap();
    //     }
    //     out.write_u8(END_ID).unwrap();

    //     std::fs::write("tests/inttest1023.nbt", out).unwrap();
    // }

    // #[test]
    // fn generate_stringtest() {
    //     let mut out = Vec::new();
    //     out.write_u8(COMPOUND_ID).unwrap();
    //     out.write_u16::<BE>(0).unwrap();
    //     out.write_u8(LIST_ID).unwrap();
    //     out.write_u16::<BE>(0).unwrap();
    //     out.write_u8(STRING_ID).unwrap();
    //     out.write_i32::<BE>(16).unwrap();
    //     out.extend_from_slice(&std::fs::read("tests/stringtest.nbt").unwrap().as_slice()[13..]);
    //     out.write_u8(END_ID).unwrap();
    //     std::fs::write("tests/stringtest2.nbt", out).unwrap();
    // }
}
