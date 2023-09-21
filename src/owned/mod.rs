//! The owned variant of NBT. This is useful if you're writing data from scratch or if you can't keep a reference to the original data.

pub mod compound;
pub mod list;

use std::{io::Cursor, ops::Deref};

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_string, read_u32, write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID,
        END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH,
        SHORT_ID, STRING_ID,
    },
    mutf8::Mutf8String,
    Error, Mutf8Str,
};

use self::{compound::CompoundTag, list::ListTag};

/// A complete NBT container. This contains a name and a compound tag.
#[derive(Debug, Clone, PartialEq)]
pub struct Nbt {
    name: Mutf8String,
    tag: CompoundTag,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionalNbt {
    Some(Nbt),
    None,
}

impl OptionalNbt {
    /// Reads NBT from the given data. Returns `Ok(None)` if there is no data.
    pub fn read(data: &mut Cursor<&[u8]>) -> Result<OptionalNbt, Error> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if root_type == END_ID {
            return Ok(OptionalNbt::None);
        }
        if root_type != COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type));
        }
        let name = read_string(data)?.to_owned();
        let tag = CompoundTag::new(data, 0)?;

        Ok(OptionalNbt::Some(Nbt { name, tag }))
    }

    pub fn unwrap(self) -> Nbt {
        match self {
            OptionalNbt::Some(nbt) => nbt,
            OptionalNbt::None => panic!("called `OptionalNbt::unwrap()` on a `None` value"),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            OptionalNbt::Some(_) => true,
            OptionalNbt::None => false,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
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
    /// Writes the NBT to the given buffer.
    pub fn write(&self, data: &mut Vec<u8>) {
        data.push(COMPOUND_ID);
        write_string(data, &self.name);
        self.tag.write(data);
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

    use byteorder::{WriteBytesExt, BE};
    use flate2::read::GzDecoder;

    use crate::common::{INT_ID, LIST_ID, LONG_ID};

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = OptionalNbt::read(&mut Cursor::new(include_bytes!(
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
        let nbt = OptionalNbt::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

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
        let nbt = OptionalNbt::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

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
        let nbt = OptionalNbt::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

        let mut out = Vec::new();
        nbt.write(&mut out);
        let nbt = OptionalNbt::read(&mut Cursor::new(&out)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn inttest_1023() {
        let nbt = OptionalNbt::read(&mut Cursor::new(include_bytes!(
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

        let nbt = OptionalNbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = OptionalNbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = OptionalNbt::read(&mut Cursor::new(&data)).unwrap().unwrap();
        let ints = nbt.list("").unwrap().longs().unwrap();
        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i64, item);
        }
        assert_eq!(ints.len(), 1023);
    }
}
