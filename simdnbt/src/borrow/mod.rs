//! The borrowed variant of NBT. This is useful if you're only reading data and you can keep a reference to the original buffer.

mod compound;
mod extra_tapes;
mod list;
mod tape;

use std::{
    fmt::{self, Debug},
    io::Cursor,
    ops::Deref,
};

use byteorder::{ReadBytesExt, BE};

use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_u32, read_with_u32_length, write_string,
        BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID,
        LIST_ID, LONG_ARRAY_ID, LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    raw_list::RawList,
    Error, Mutf8Str,
};

pub use self::{compound::NbtCompound, list::NbtList};
use self::{
    extra_tapes::ExtraTapes,
    tape::{MainTape, TapeElement, TapeTagKind, TapeTagValue, UnalignedU16},
};

/// Read a normal root NBT compound. This is either empty or has a name and compound tag.
///
/// Returns `Ok(Nbt::None)` if there is no data.
pub fn read<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
    Nbt::read(data)
}
/// Read a root NBT compound, but without reading the name. This is used in Minecraft when reading
/// NBT over the network.
///
/// This is similar to [`read_tag`], but returns an [`Nbt`] instead (guaranteeing it'll be either
/// empty or a compound).
pub fn read_unnamed<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
    Nbt::read_unnamed(data)
}
/// Read a compound tag. This may have any number of items.
pub fn read_compound<'a>(data: &mut Cursor<&'a [u8]>) -> Result<BaseNbtCompound<'a>, Error> {
    let mut tapes = Tapes::new();
    NbtCompound::read(data, &mut tapes)?;
    Ok(BaseNbtCompound { tapes })
}
/// Read an NBT tag, without reading its name. This may be any type of tag except for an end tag. If you need to be able to
/// handle end tags, use [`read_optional_tag`].
pub fn read_tag<'a>(data: &mut Cursor<&'a [u8]>) -> Result<BaseNbtTag<'a>, Error> {
    let mut tapes = Tapes::new();
    NbtTag::read(data, &mut tapes)?;
    Ok(BaseNbtTag { tapes })
}
/// Read any NBT tag, without reading its name. This may be any type of tag, including an end tag.
///
/// Returns `Ok(None)` if there is no data.
pub fn read_optional_tag<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Option<BaseNbtTag<'a>>, Error> {
    let mut tapes = Tapes::new();
    let tag = NbtTag::read_optional(data, &mut tapes)?;
    Ok(if tag {
        Some(BaseNbtTag { tapes })
    } else {
        None
    })
}

#[derive(Default)]
pub(crate) struct Tapes<'a> {
    main: MainTape,
    extra: ExtraTapes<'a>,
}
impl<'a> Tapes<'a> {
    fn new() -> Self {
        Self::default()
    }
}
impl Debug for Tapes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tapes").finish()
    }
}

/// A complete NBT container. This contains a name and a compound tag.
pub struct BaseNbt<'a> {
    name: &'a Mutf8Str,
    tapes: Tapes<'a>,
}
impl<'a> BaseNbt<'a> {
    pub fn compound<'tape>(&'a self) -> NbtCompound<'a, 'tape>
    where
        'a: 'tape,
    {
        NbtCompound {
            element: self.tapes.main.elements.as_ptr(),
            extra_tapes: &self.tapes.extra,
        }
    }

    /// Get the name of the NBT compound. This is often an empty string.
    pub fn name(&self) -> &'a Mutf8Str {
        self.name
    }
}
impl<'a> Debug for BaseNbt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseNbt").finish()
    }
}

/// A nameless NBT container. This only contains a compound tag. This contains a `TagAllocator`,
/// so it can exist independently from a [`BaseNbt`].
pub struct BaseNbtCompound<'a> {
    tapes: Tapes<'a>,
}

/// A nameless NBT tag.
pub struct BaseNbtTag<'a> {
    tapes: Tapes<'a>,
}
impl<'a> BaseNbtTag<'a> {
    pub fn compound<'tape>(&'a self) -> NbtCompound<'a, 'tape>
    where
        'a: 'tape,
    {
        NbtCompound {
            element: self.tapes.main.elements.as_ptr(),
            extra_tapes: &self.tapes.extra,
        }
    }
}

/// Either a complete NBT container, or nothing.
#[derive(Debug, PartialEq, Default)]
pub enum Nbt<'a> {
    Some(BaseNbt<'a>),
    #[default]
    None,
}

impl<'a> Nbt<'a> {
    /// Reads NBT from the given data. Returns `Ok(Nbt::None)` if there is no data.
    fn read(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if root_type == END_ID {
            return Ok(Nbt::None);
        }
        if root_type != COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type));
        }

        let mut tapes = Tapes::new();

        let name = read_string(data)?;
        NbtCompound::read(data, &mut tapes)?;

        Ok(Nbt::Some(BaseNbt { name, tapes }))
    }

    fn read_unnamed(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if root_type == END_ID {
            return Ok(Nbt::None);
        }
        if root_type != COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type));
        }
        let mut tapes = Tapes::new();

        NbtCompound::read(data, &mut tapes)?;

        Ok(Nbt::Some(BaseNbt {
            name: Mutf8Str::from_slice(&[]),
            tapes,
        }))
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        todo!();
        // match self {
        //     Nbt::Some(nbt) => nbt.write(data),
        //     Nbt::None => {
        //         data.push(END_ID);
        //     }
        // }
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

impl PartialEq for BaseNbt<'_> {
    fn eq(&self, other: &Self) -> bool {
        todo!();
        // we don't need to compare the tapes since comparing `tag` will
        // still compare the values of the tags
        // self.name == other.name && self.tag == other.tag
    }
}

impl<'a> BaseNbt<'a> {
    pub fn write(&self, data: &mut Vec<u8>) {
        // data.push(COMPOUND_ID);
        // write_string(data, self.name);
        // self.tag.write(data);
        // data.push(END_ID);
    }
}

#[derive(Debug)]
pub struct NbtTag<'a: 'tape, 'tape> {
    element: *const TapeElement,
    extra_tapes: &'tape ExtraTapes<'a>,
}

impl<'a: 'tape, 'tape> NbtTag<'a, 'tape> {
    /// Get the numerical ID of the tag type.
    #[inline]
    pub fn id(&self) -> u8 {
        match self.element().0 {
            TapeTagKind::Byte => BYTE_ID,
            TapeTagKind::Short => SHORT_ID,
            TapeTagKind::Int => INT_ID,
            TapeTagKind::Long => LONG_ID,
            TapeTagKind::Float => FLOAT_ID,
            TapeTagKind::Double => DOUBLE_ID,
            TapeTagKind::ByteArray => BYTE_ARRAY_ID,
            TapeTagKind::String => STRING_ID,
            TapeTagKind::Compound => COMPOUND_ID,
            TapeTagKind::IntArray => INT_ARRAY_ID,
            TapeTagKind::LongArray => LONG_ARRAY_ID,
            t if t.is_list() => LIST_ID,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn read_with_type(
        data: &mut Cursor<&'a [u8]>,
        tapes: &'tape mut Tapes<'a>,
        tag_type: u8,
        depth: usize,
    ) -> Result<(), Error> {
        match tag_type {
            BYTE_ID => {
                let byte = data.read_i8().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Byte, TapeTagValue { byte }),
                });
                Ok(())
            }
            SHORT_ID => {
                let short = data.read_i16::<BE>().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Short, TapeTagValue { short }),
                });
                Ok(())
            }
            INT_ID => {
                let int = data.read_i32::<BE>().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Int, TapeTagValue { int }),
                });
                Ok(())
            }
            LONG_ID => {
                let long = data.read_i64::<BE>().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Long, TapeTagValue { long: () }),
                });
                tapes.main.elements.push(TapeElement { long });
                Ok(())
            }
            FLOAT_ID => {
                let float = data.read_f32::<BE>().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Float, TapeTagValue { float }),
                });
                Ok(())
            }
            DOUBLE_ID => {
                let double = data.read_f64::<BE>().map_err(|_| Error::UnexpectedEof)?;
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::Double, TapeTagValue { double: () }),
                });
                tapes.main.elements.push(TapeElement { double });
                Ok(())
            }
            BYTE_ARRAY_ID => {
                let byte_array_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 1)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::ByteArray,
                        TapeTagValue {
                            byte_array: byte_array_pointer.into(),
                        },
                    ),
                });
                Ok(())
            }
            STRING_ID => {
                let string_pointer = data.get_ref().as_ptr() as u64 + data.position();

                // assert that the top 8 bits of the pointer are 0 (because we rely on this)
                debug_assert_eq!(string_pointer >> 56, 0);

                read_string(data)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::String,
                        TapeTagValue {
                            string: string_pointer.into(),
                        },
                    ),
                });
                Ok(())
            }
            LIST_ID => NbtList::read(data, tapes, depth + 1),
            COMPOUND_ID => NbtCompound::read_with_depth(data, tapes, depth + 1),
            INT_ARRAY_ID => {
                let int_array_pointer = data.get_ref().as_ptr() as u64 + data.position();
                // let int_array = read_int_array(data)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::IntArray,
                        TapeTagValue {
                            int_array: int_array_pointer.into(),
                        },
                    ),
                });
                Ok(())
            }
            LONG_ARRAY_ID => {
                let long_array_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_long_array(data)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::LongArray,
                        TapeTagValue {
                            long_array: long_array_pointer.into(),
                        },
                    ),
                });
                Ok(())
            }
            _ => Err(Error::UnknownTagId(tag_type)),
        }
    }

    fn read(data: &mut Cursor<&'a [u8]>, tapes: &'tape mut Tapes<'a>) -> Result<(), Error> {
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        Self::read_with_type(data, tapes, tag_type, 0)
    }

    fn read_optional(
        data: &mut Cursor<&'a [u8]>,
        tapes: &'tape mut Tapes<'a>,
    ) -> Result<bool, Error> {
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if tag_type == END_ID {
            return Ok(false);
        }
        Self::read_with_type(data, tapes, tag_type, 0)?;
        Ok(true)
    }

    pub fn byte(&self) -> Option<i8> {
        // match self {
        //     NbtTag::Byte(byte) => Some(*byte),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::Byte {
            return None;
        }
        Some(unsafe { value.byte })
    }
    pub fn short(&self) -> Option<i16> {
        // match self {
        //     NbtTag::Short(short) => Some(*short),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::Short {
            return None;
        }
        Some(unsafe { value.short })
    }
    pub fn int(&self) -> Option<i32> {
        // match self {
        //     NbtTag::Int(int) => Some(*int),
        //     _ => None,
        // }
        let (kind, value) = unsafe { (*self.element).kind };
        if kind != TapeTagKind::Int {
            return None;
        }
        Some(unsafe { value.int })
    }
    pub fn long(&self) -> Option<i64> {
        // match self {
        //     NbtTag::Long(long) => Some(*long),
        //     _ => None,
        // }
        let (kind, _) = self.element();
        if kind != TapeTagKind::Long {
            return None;
        }
        // the value is in the next element because longs are too big to fit in a single element
        let value = unsafe { (self.element as *const TapeElement).add(1) };
        Some(unsafe { (*value).long })
    }
    pub fn float(&self) -> Option<f32> {
        // match self {
        //     NbtTag::Float(float) => Some(*float),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::Float {
            return None;
        }
        Some(unsafe { value.float })
    }
    pub fn double(&self) -> Option<f64> {
        // match self {
        //     NbtTag::Double(double) => Some(*double),
        //     _ => None,
        // }
        let (kind, _) = self.element();
        if kind != TapeTagKind::Double {
            return None;
        }
        // the value is in the next element because doubles are too big to fit in a single element
        let value = unsafe { (self.element as *const TapeElement).add(1) };
        Some(unsafe { (*value).double })
    }
    pub fn byte_array(&self) -> Option<&'a [u8]> {
        // match self {
        //     NbtTag::ByteArray(byte_array) => Some(byte_array),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::ByteArray {
            return None;
        }
        let length_ptr = unsafe { u64::from(value.byte_array) as *const u32 };
        let length = unsafe { *length_ptr as usize };
        let data_ptr = unsafe { length_ptr.add(1) as *const u8 };
        Some(unsafe { std::slice::from_raw_parts(data_ptr, length) })
    }
    pub fn string(&self) -> Option<&'a Mutf8Str> {
        // match self {
        //     NbtTag::String(string) => Some(string),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::String {
            return None;
        }
        let length_ptr = unsafe { u64::from(value.string) as usize as *const UnalignedU16 };
        let length = unsafe { u16::from(*length_ptr).swap_bytes() as usize };
        let data_ptr = unsafe { length_ptr.add(1) as *const u8 };
        Some(unsafe { Mutf8Str::from_slice(std::slice::from_raw_parts(data_ptr, length)) })
    }
    pub fn list(&self) -> Option<NbtList<'a, 'tape>> {
        // match self {
        //     NbtTag::List(list) => Some(list),
        //     _ => None,
        // }
        let (kind, _) = self.element();
        if !kind.is_list() {
            return None;
        }

        Some(NbtList {
            element: self.element,
            extra_tapes: self.extra_tapes,
        })
    }
    pub fn compound(&self) -> Option<NbtCompound<'a, 'tape>> {
        // match self {
        //     NbtTag::Compound(compound) => Some(compound),
        //     _ => None,
        // }
        let (kind, _) = self.element();
        if kind != TapeTagKind::Compound {
            return None;
        }

        Some(NbtCompound {
            element: self.element,
            extra_tapes: self.extra_tapes,
        })
    }
    pub fn int_array(&self) -> Option<Vec<i32>> {
        list::u32_prefixed_list_to_vec(TapeTagKind::IntArray, self.element)
    }
    pub fn long_array(&self) -> Option<Vec<i64>> {
        list::u32_prefixed_list_to_vec(TapeTagKind::LongArray, self.element)
    }

    /// Get the tape element kind and value for this tag.
    fn element(&self) -> (TapeTagKind, TapeTagValue) {
        unsafe { (*self.element).kind }
    }

    pub fn to_owned(&self) -> crate::owned::NbtTag {
        todo!()
        // match self {
        //     NbtTag::Byte(byte) => crate::owned::NbtTag::Byte(*byte),
        //     NbtTag::Short(short) => crate::owned::NbtTag::Short(*short),
        //     NbtTag::Int(int) => crate::owned::NbtTag::Int(*int),
        //     NbtTag::Long(long) => crate::owned::NbtTag::Long(*long),
        //     NbtTag::Float(float) => crate::owned::NbtTag::Float(*float),
        //     NbtTag::Double(double) => crate::owned::NbtTag::Double(*double),
        //     NbtTag::ByteArray(byte_array) => crate::owned::NbtTag::ByteArray(byte_array.to_vec()),
        //     NbtTag::String(string) => crate::owned::NbtTag::String((*string).to_owned()),
        //     NbtTag::List(list) => crate::owned::NbtTag::List(list.to_owned()),
        //     NbtTag::Compound(compound) => crate::owned::NbtTag::Compound(compound.to_owned()),
        //     NbtTag::IntArray(int_array) => crate::owned::NbtTag::IntArray(int_array.to_vec()),
        //     NbtTag::LongArray(long_array) => crate::owned::NbtTag::LongArray(long_array.to_vec()),
        // }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use byteorder::WriteBytesExt;
    use flate2::read::GzDecoder;

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = Nbt::read(&mut Cursor::new(include_bytes!(
            "../../tests/hello_world.nbt"
        )))
        .unwrap()
        .unwrap();

        assert_eq!(
            nbt.compound().string("name"),
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

        assert_eq!(nbt.compound().int("PersistentId"), Some(1946940766));
        assert_eq!(
            nbt.compound()
                .list("Rotation")
                .unwrap()
                .floats()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn read_complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::read(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        assert_eq!(
            nbt.compound().float("foodExhaustionLevel").unwrap() as u32,
            2
        );
        assert_eq!(
            nbt.compound()
                .list("Rotation")
                .unwrap()
                .floats()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn read_hypixel() {
        let src = include_bytes!("../../tests/hypixel.nbt").to_vec();
        let _nbt = Nbt::read(&mut Cursor::new(&src[..])).unwrap().unwrap();
    }

    #[test]
    fn read_write_complex_player() {
        return;
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = Nbt::read(&mut Cursor::new(&decoded_src)).unwrap().unwrap();

        let mut out = Vec::new();
        nbt.write(&mut out);
        let nbt = Nbt::read(&mut Cursor::new(&out)).unwrap().unwrap();

        assert_eq!(
            nbt.compound().float("foodExhaustionLevel").unwrap() as u32,
            2
        );
        assert_eq!(
            nbt.compound()
                .list("Rotation")
                .unwrap()
                .floats()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn inttest_1023() {
        let nbt = Nbt::read(&mut Cursor::new(include_bytes!(
            "../../tests/inttest1023.nbt"
        )))
        .unwrap()
        .unwrap();

        let ints = nbt.compound().list("").unwrap().ints().unwrap();

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
        let ints = nbt.compound().list("").unwrap().ints().unwrap();
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
        let ints = nbt.compound().list("").unwrap().ints().unwrap();
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
        let ints = nbt.compound().list("").unwrap().longs().unwrap();
        for (i, &item) in ints.iter().enumerate() {
            assert_eq!(i as i64, item);
        }
        assert_eq!(ints.len(), 1023);
    }

    #[test]
    fn compound_eof() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap(); // root type
        data.write_u16::<BE>(0).unwrap(); // root name length
        data.write_u8(COMPOUND_ID).unwrap(); // first element type
        data.write_u16::<BE>(0).unwrap(); // first element name length
                                          // eof

        let res = Nbt::read(&mut Cursor::new(&data));
        assert_eq!(res, Err(Error::UnexpectedEof));
    }

    #[test]
    fn read_complexplayer_with_given_alloc() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();

        let mut decoded_src_as_tag = Vec::new();
        decoded_src_as_tag.push(COMPOUND_ID);
        decoded_src_as_tag.extend_from_slice(&decoded_src);
        decoded_src_as_tag.push(END_ID);

        let nbt = super::read_tag(&mut Cursor::new(&decoded_src_as_tag)).unwrap();
        let nbt = nbt.compound().compound("").unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }
}
