//! The borrowed variant of NBT. This is useful if you're only reading data and you can keep a reference to the original buffer.

mod compound;
mod list;

use std::{io::Cursor, ops::Deref};

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_string, read_u32, write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID,
        END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH,
        SHORT_ID, STRING_ID,
    },
    raw_list::RawList,
    Error, Mutf8Str,
};

pub use self::{compound::NbtCompound, list::NbtList};

/// A complete NBT container. This contains a name and a compound tag.
#[derive(Debug, PartialEq)]
pub struct BaseNbt<'a> {
    name: &'a Mutf8Str,
    tag: NbtCompound<'a>,
}

#[derive(Debug, PartialEq, Default)]
pub enum Nbt<'a> {
    Some(BaseNbt<'a>),
    #[default]
    None,
}

impl<'a> Nbt<'a> {
    /// Reads NBT from the given data. Returns `Ok(None)` if there is no data.
    pub fn read(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if root_type == END_ID {
            return Ok(Nbt::None);
        }
        if root_type != COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type));
        }
        let name = read_string(data)?;
        let tag = NbtCompound::read_with_depth(data, 0)?;

        Ok(Nbt::Some(BaseNbt { name, tag }))
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        match self {
            Nbt::Some(nbt) => nbt.write(data),
            Nbt::None => {
                data.push(END_ID);
            }
        }
    }

    pub fn unwrap(self) -> BaseNbt<'a> {
        match self {
            Nbt::Some(nbt) => nbt,
            Nbt::None => panic!("called `OptionalNbt::unwrap()` on a `None` value"),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Nbt::Some(_) => true,
            Nbt::None => false,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl<'a> BaseNbt<'a> {
    /// Get the name of the NBT compound. This is often an empty string.
    pub fn name(&self) -> &'a Mutf8Str {
        self.name
    }
}
impl<'a> Deref for BaseNbt<'a> {
    type Target = NbtCompound<'a>;

    fn deref(&self) -> &Self::Target {
        &self.tag
    }
}

impl<'a> BaseNbt<'a> {
    pub fn write(&self, data: &mut Vec<u8>) {
        data.push(COMPOUND_ID);
        write_string(data, self.name);
        self.tag.write(data);
        data.push(END_ID);
    }
}

/// A single NBT tag.
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum NbtTag<'a> {
    Byte(i8) = BYTE_ID,
    Short(i16) = SHORT_ID,
    Int(i32) = INT_ID,
    Long(i64) = LONG_ID,
    Float(f32) = FLOAT_ID,
    Double(f64) = DOUBLE_ID,
    ByteArray(&'a [u8]) = BYTE_ARRAY_ID,
    String(&'a Mutf8Str) = STRING_ID,
    List(NbtList<'a>) = LIST_ID,
    Compound(NbtCompound<'a>) = COMPOUND_ID,
    IntArray(RawList<'a, i32>) = INT_ARRAY_ID,
    LongArray(RawList<'a, i64>) = LONG_ARRAY_ID,
}
impl<'a> NbtTag<'a> {
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
            NbtTag::Byte(byte) => Some(*byte),
            _ => None,
        }
    }
    pub fn short(&self) -> Option<i16> {
        match self {
            NbtTag::Short(short) => Some(*short),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i32> {
        match self {
            NbtTag::Int(int) => Some(*int),
            _ => None,
        }
    }
    pub fn long(&self) -> Option<i64> {
        match self {
            NbtTag::Long(long) => Some(*long),
            _ => None,
        }
    }
    pub fn float(&self) -> Option<f32> {
        match self {
            NbtTag::Float(float) => Some(*float),
            _ => None,
        }
    }
    pub fn double(&self) -> Option<f64> {
        match self {
            NbtTag::Double(double) => Some(*double),
            _ => None,
        }
    }
    pub fn byte_array(&self) -> Option<&[u8]> {
        match self {
            NbtTag::ByteArray(byte_array) => Some(byte_array),
            _ => None,
        }
    }
    pub fn string(&self) -> Option<&Mutf8Str> {
        match self {
            NbtTag::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn list(&self) -> Option<&NbtList<'a>> {
        match self {
            NbtTag::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn compound(&self) -> Option<&NbtCompound<'a>> {
        match self {
            NbtTag::Compound(compound) => Some(compound),
            _ => None,
        }
    }
    pub fn int_array(&self) -> Option<Vec<i32>> {
        match self {
            NbtTag::IntArray(int_array) => Some(int_array.to_vec()),
            _ => None,
        }
    }
    pub fn long_array(&self) -> Option<Vec<i64>> {
        match self {
            NbtTag::LongArray(long_array) => Some(long_array.to_vec()),
            _ => None,
        }
    }

    pub fn to_owned(&self) -> crate::owned::NbtTag {
        match self {
            NbtTag::Byte(byte) => crate::owned::NbtTag::Byte(*byte),
            NbtTag::Short(short) => crate::owned::NbtTag::Short(*short),
            NbtTag::Int(int) => crate::owned::NbtTag::Int(*int),
            NbtTag::Long(long) => crate::owned::NbtTag::Long(*long),
            NbtTag::Float(float) => crate::owned::NbtTag::Float(*float),
            NbtTag::Double(double) => crate::owned::NbtTag::Double(*double),
            NbtTag::ByteArray(byte_array) => crate::owned::NbtTag::ByteArray(byte_array.to_vec()),
            NbtTag::String(string) => crate::owned::NbtTag::String((*string).to_owned()),
            NbtTag::List(list) => crate::owned::NbtTag::List(list.to_owned()),
            NbtTag::Compound(compound) => crate::owned::NbtTag::Compound(compound.to_owned()),
            NbtTag::IntArray(int_array) => crate::owned::NbtTag::IntArray(int_array.to_vec()),
            NbtTag::LongArray(long_array) => crate::owned::NbtTag::LongArray(long_array.to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use byteorder::{WriteBytesExt, BE};
    use flate2::read::GzDecoder;

    use crate::common::{INT_ID, LIST_ID, LONG_ID};

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = Nbt::read(&mut Cursor::new(include_bytes!(
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
        let nbt = Nbt::read(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        assert_eq!(nbt.int("PersistentId"), Some(1946940766));
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn read_complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::read(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

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
        let nbt = Nbt::read(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        let mut out = Vec::new();
        nbt.write(&mut out);
        let nbt = Nbt::read(&mut Cursor::new(&out)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn inttest_1023() {
        let nbt = Nbt::read(&mut Cursor::new(include_bytes!(
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

        let nbt = Nbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = Nbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = Nbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
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
