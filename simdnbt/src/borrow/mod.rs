//! The borrowed variant of NBT. This is useful if you're only reading data and
//! you can keep a reference to the original buffer.

mod compound;
mod extra_tapes;
mod list;
mod tape;

use std::{
    fmt::{self, Debug},
    io::Cursor,
};

use byteorder::ReadBytesExt;
use compound::ParsingStackElementKind;
use tape::{UnalignedU16, UnalignedU32, UnalignedU64};

use self::{
    compound::{read_tag_in_compound, ParsingStack, ParsingStackElement},
    extra_tapes::ExtraTapes,
    list::{read_compound_in_list, read_list_in_list},
    tape::{MainTape, TapeElement, TapeTagKind},
};
pub use self::{
    compound::{NbtCompound, NbtCompoundIter},
    list::{NbtCompoundList, NbtCompoundListIter, NbtList, NbtListList, NbtListListIter},
};
use crate::{
    common::{
        read_string, write_string, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID,
        FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    reader::{Reader, ReaderFromCursor},
    Error, Mutf8Str,
};

/// Read a normal root NBT compound. This is either empty or has a name and
/// compound tag.
///
/// Returns `Ok(Nbt::None)` if there is no data.
pub fn read<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
    let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if root_type == END_ID {
        return Ok(Nbt::None);
    }
    if root_type != COMPOUND_ID {
        return Err(Error::InvalidRootType(root_type));
    }
    // our Reader type is faster than Cursor
    let mut data = ReaderFromCursor::new(data);
    let name = read_string(&mut data)?;

    let mut tapes = Tapes::new();
    tapes.main.push(TapeElement::new_with_approx_len_and_offset(
        TapeTagKind::Compound,
        // these get overwritten later
        0,
        0,
    ));

    let mut stack = ParsingStack::new();
    stack.push(ParsingStackElement::compound(0))?;

    read_with_stack(&mut data, &mut tapes, &mut stack)?;

    Ok(Nbt::Some(BaseNbt { name, tapes }))
}
/// Read a root NBT compound, but without reading the name. This is used in
/// Minecraft when reading NBT over the network.
///
/// This is similar to [`read_tag`], but returns an [`Nbt`] instead
/// (guaranteeing it'll be either empty or a compound).
pub fn read_unnamed<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Nbt<'a>, Error> {
    let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if root_type == END_ID {
        return Ok(Nbt::None);
    }
    if root_type != COMPOUND_ID {
        return Err(Error::InvalidRootType(root_type));
    }
    let name = Mutf8Str::from_slice(&[]);
    let BaseNbtCompound { tapes } = read_compound(data)?;
    Ok(Nbt::Some(BaseNbt { name, tapes }))
}
/// Read a compound tag. This may have any number of items.
pub fn read_compound<'a>(data: &mut Cursor<&'a [u8]>) -> Result<BaseNbtCompound<'a>, Error> {
    let mut tapes = Tapes::new();
    let mut stack = ParsingStack::new();

    let mut data = ReaderFromCursor::new(data);

    stack.push(ParsingStackElement::compound(0))?;

    tapes.main.push(TapeElement::new_with_approx_len_and_offset(
        TapeTagKind::Compound,
        // these get overwritten later
        0,
        0,
    ));

    read_with_stack(&mut data, &mut tapes, &mut stack)?;

    Ok(BaseNbtCompound { tapes })
}
/// Read an NBT tag, without reading its name. This may be any type of tag
/// except for an end tag. If you need to be able to handle end tags, use
/// [`read_optional_tag`].
pub fn read_tag<'a>(data: &mut Cursor<&'a [u8]>) -> Result<BaseNbtTag<'a>, Error> {
    let mut tapes = Tapes::new();
    let mut stack = ParsingStack::new();

    let mut data = ReaderFromCursor::new(data);

    let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if tag_type == END_ID {
        return Err(Error::InvalidRootType(0));
    }
    compound::read_tag(&mut data, &mut tapes, &mut stack, tag_type)?;
    read_with_stack(&mut data, &mut tapes, &mut stack)?;

    Ok(BaseNbtTag { tapes })
}
/// Read any NBT tag, without reading its name. This may be any type of tag,
/// including an end tag.
///
/// Returns `Ok(None)` if there is no data.
pub fn read_optional_tag<'a>(data: &mut Cursor<&'a [u8]>) -> Result<Option<BaseNbtTag<'a>>, Error> {
    let mut tapes = Tapes::new();
    let mut stack = ParsingStack::new();

    let mut data = ReaderFromCursor::new(data);

    let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if tag_type == END_ID {
        return Ok(None);
    }
    compound::read_tag(&mut data, &mut tapes, &mut stack, tag_type)?;
    read_with_stack(&mut data, &mut tapes, &mut stack)?;

    Ok(Some(BaseNbtTag { tapes }))
}

fn read_with_stack<'a>(
    data: &mut Reader<'a>,
    tapes: &mut Tapes<'a>,
    stack: &mut ParsingStack,
) -> Result<(), Error> {
    while !stack.is_empty() {
        let top = stack.peek_mut();
        match top.kind {
            ParsingStackElementKind::Compound => read_tag_in_compound(data, tapes, stack)?,
            ParsingStackElementKind::ListOfCompounds => read_compound_in_list(data, tapes, stack)?,
            ParsingStackElementKind::ListOfLists => read_list_in_list(data, tapes, stack)?,
        }
    }

    Ok(())
}

#[derive(Default)]
pub(crate) struct Tapes<'a> {
    main: MainTape,
    extra: ExtraTapes<'a>,
}
impl Tapes<'_> {
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
    #[inline]
    pub fn as_compound<'tape>(&'a self) -> NbtCompound<'a, 'tape>
    where
        'a: 'tape,
    {
        NbtCompound {
            element: self.tapes.main.as_ptr(),
            extra_tapes: &self.tapes.extra,
        }
    }

    /// Get the name of the NBT compound. This is often an empty string.
    pub fn name(&self) -> &'a Mutf8Str {
        self.name
    }

    pub fn get<'tape>(&'a self, key: &str) -> Option<NbtTag<'a, 'tape>> {
        self.as_compound().get(key)
    }
    /// Returns whether there is a tag with the given name.
    pub fn contains(&'a self, key: &str) -> bool {
        self.as_compound().contains(key)
    }
    pub fn byte(&self, name: &str) -> Option<i8> {
        self.as_compound().byte(name)
    }
    pub fn short(&self, name: &str) -> Option<i16> {
        self.as_compound().short(name)
    }
    pub fn int(&self, name: &str) -> Option<i32> {
        self.as_compound().int(name)
    }
    pub fn long(&self, name: &str) -> Option<i64> {
        self.as_compound().long(name)
    }
    pub fn float(&self, name: &str) -> Option<f32> {
        self.as_compound().float(name)
    }
    pub fn double(&self, name: &str) -> Option<f64> {
        self.as_compound().double(name)
    }
    pub fn byte_array(&'a self, name: &str) -> Option<&'a [u8]> {
        self.as_compound().byte_array(name)
    }
    pub fn string(&'a self, name: &str) -> Option<&'a Mutf8Str> {
        self.as_compound().string(name)
    }
    pub fn list<'tape>(&'a self, name: &str) -> Option<NbtList<'a, 'tape>> {
        self.as_compound().list(name)
    }
    pub fn compound<'tape>(&'a self, name: &str) -> Option<NbtCompound<'a, 'tape>> {
        self.as_compound().compound(name)
    }
    pub fn int_array(&self, name: &str) -> Option<Vec<i32>> {
        self.as_compound().int_array(name)
    }
    pub fn long_array(&self, name: &str) -> Option<Vec<i64>> {
        self.as_compound().long_array(name)
    }
}

impl Debug for BaseNbt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseNbt").finish()
    }
}

/// A nameless NBT container. This only contains a compound tag. This contains a
/// `TagAllocator`, so it can exist independently from a [`BaseNbt`].
pub struct BaseNbtCompound<'a> {
    tapes: Tapes<'a>,
}
impl<'a, 'tape> From<&'a BaseNbtCompound<'a>> for NbtCompound<'a, 'tape> {
    fn from(compound: &'a BaseNbtCompound<'a>) -> Self
    where
        'a: 'tape,
    {
        NbtCompound {
            element: compound.tapes.main.as_ptr(),
            extra_tapes: &compound.tapes.extra,
        }
    }
}

/// A nameless NBT tag.
pub struct BaseNbtTag<'a> {
    tapes: Tapes<'a>,
}
impl<'a> BaseNbtTag<'a> {
    pub fn as_tag<'tape>(&'a self) -> NbtTag<'a, 'tape>
    where
        'a: 'tape,
    {
        NbtTag {
            element: self.tapes.main.as_ptr(),
            extra_tapes: &self.tapes.extra,
        }
    }
}
impl<'a> From<&'a BaseNbtTag<'a>> for NbtTag<'a, '_> {
    fn from(tag: &'a BaseNbtTag<'a>) -> Self {
        tag.as_tag()
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

impl PartialEq for BaseNbt<'_> {
    fn eq(&self, other: &Self) -> bool {
        // we don't need to compare the tapes since comparing `tag` will
        // still compare the values of the tags
        self.name == other.name && self.as_compound() == other.as_compound()
    }
}

impl BaseNbt<'_> {
    pub fn write(&self, data: &mut Vec<u8>) {
        data.push(COMPOUND_ID);
        write_string(data, self.name);
        self.as_compound().write(data);
        data.push(END_ID);
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
        match self.element().kind() {
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

    pub fn byte(&self) -> Option<i8> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Byte)?;
        Some(el.u8() as i8)
    }
    pub fn short(&self) -> Option<i16> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Short)?;
        Some(el.u16() as i16)
    }
    pub fn int(&self) -> Option<i32> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Int)?;
        Some(el.u32() as i32)
    }
    pub fn long(&self) -> Option<i64> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Long)?;
        // longs are 64 bits so since we can't fit the kind and value in a
        // single element, we store a pointer to them instead
        let long_ptr = el.ptr::<UnalignedU64>();
        Some(u64::from(unsafe { *long_ptr }).to_be() as i64)
    }
    pub fn float(&self) -> Option<f32> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Float)?;
        Some(f32::from_bits(el.u32()))
    }
    pub fn double(&self) -> Option<f64> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Double)?;
        // see the comment above for longs, doubles are also 64 bits so the same applies
        let double_ptr = el.ptr::<UnalignedU64>();
        Some(f64::from_bits(u64::from(unsafe { *double_ptr }).to_be()))
    }
    pub fn byte_array(&self) -> Option<&'a [u8]> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::ByteArray)?;
        let length_ptr = el.ptr::<UnalignedU32>();
        let length = u32::from(unsafe { *length_ptr });
        #[cfg(target_endian = "little")]
        let length = length.swap_bytes();
        let data_ptr = unsafe { length_ptr.add(1) as *const u8 };
        Some(unsafe { std::slice::from_raw_parts(data_ptr, length as usize) })
    }
    pub fn string(&self) -> Option<&'a Mutf8Str> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::String)?;
        let length_ptr = el.ptr::<UnalignedU16>();
        let length = u16::from(unsafe { *length_ptr });
        #[cfg(target_endian = "little")]
        let length = length.swap_bytes();
        let data_ptr = unsafe { length_ptr.add(1) as *const u8 };
        Some(unsafe { Mutf8Str::from_slice(std::slice::from_raw_parts(data_ptr, length as usize)) })
    }
    pub fn list(&self) -> Option<NbtList<'a, 'tape>> {
        let el = self.element();
        if !el.kind().is_list() {
            return None;
        }

        Some(NbtList {
            element: self.element,
            extra_tapes: self.extra_tapes,
        })
    }
    pub fn compound(&self) -> Option<NbtCompound<'a, 'tape>> {
        let el = self.element();
        ensure_kind(el, TapeTagKind::Compound)?;

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

    /// Get the tape element for this tag.
    fn element(&self) -> TapeElement {
        unsafe { *self.element }
    }

    pub fn to_owned(&self) -> crate::owned::NbtTag {
        let el = self.element();

        match el.kind() {
            TapeTagKind::Byte => crate::owned::NbtTag::Byte(self.byte().unwrap()),
            TapeTagKind::Short => crate::owned::NbtTag::Short(self.short().unwrap()),
            TapeTagKind::Int => crate::owned::NbtTag::Int(self.int().unwrap()),
            TapeTagKind::Long => crate::owned::NbtTag::Long(self.long().unwrap()),
            TapeTagKind::Float => crate::owned::NbtTag::Float(self.float().unwrap()),
            TapeTagKind::Double => crate::owned::NbtTag::Double(self.double().unwrap()),
            TapeTagKind::ByteArray => {
                crate::owned::NbtTag::ByteArray(self.byte_array().unwrap().to_vec())
            }
            TapeTagKind::String => crate::owned::NbtTag::String(self.string().unwrap().to_owned()),
            TapeTagKind::Compound => {
                crate::owned::NbtTag::Compound(self.compound().unwrap().to_owned())
            }
            kind if kind.is_list() => crate::owned::NbtTag::List(self.list().unwrap().to_owned()),
            TapeTagKind::IntArray => crate::owned::NbtTag::IntArray(self.int_array().unwrap()),
            TapeTagKind::LongArray => crate::owned::NbtTag::LongArray(self.long_array().unwrap()),
            _ => unreachable!(),
        }
    }
}

fn ensure_kind(el: TapeElement, other: TapeTagKind) -> Option<()> {
    if el.kind() != other {
        None
    } else {
        Some(())
    }
}

impl PartialEq for NbtTag<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        let self_el = self.element();
        let other_el = other.element();
        if self_el.kind() != other_el.kind() {
            return false;
        }
        match self_el.kind() {
            TapeTagKind::Byte => self.byte().unwrap() == other.byte().unwrap(),
            TapeTagKind::Short => self.short().unwrap() == other.short().unwrap(),
            TapeTagKind::Int => self.int().unwrap() == other.int().unwrap(),
            TapeTagKind::Long => self.long().unwrap() == other.long().unwrap(),
            TapeTagKind::Float => self.float().unwrap() == other.float().unwrap(),
            TapeTagKind::Double => self.double().unwrap() == other.double().unwrap(),
            TapeTagKind::ByteArray => self.byte_array().unwrap() == other.byte_array().unwrap(),
            TapeTagKind::String => self.string().unwrap() == other.string().unwrap(),
            TapeTagKind::Compound => self.compound().unwrap() == other.compound().unwrap(),
            TapeTagKind::IntArray => self.int_array().unwrap() == other.int_array().unwrap(),
            TapeTagKind::LongArray => self.long_array().unwrap() == other.long_array().unwrap(),
            t if t.is_list() => self.list().unwrap() == other.list().unwrap(),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use byteorder::{WriteBytesExt, BE};
    use flate2::read::GzDecoder;

    use super::*;

    #[test]
    fn hello_world() {
        let nbt = super::read(&mut Cursor::new(include_bytes!(
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
        let nbt = super::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

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
        let nbt = super::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn read_hypixel() {
        let src = include_bytes!("../../tests/hypixel.nbt").to_vec();
        let _nbt = super::read(&mut Cursor::new(&src[..])).unwrap().unwrap();
    }

    #[test]
    fn read_write_complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = super::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

        let mut out = Vec::new();
        nbt.write(&mut out);

        let nbt = super::read(&mut Cursor::new(&out)).unwrap().unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn inttest_1023() {
        let nbt = super::read(&mut Cursor::new(include_bytes!(
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

        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
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

        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
        let ints = nbt.list("").unwrap().longs().unwrap();
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

        let res = super::read(&mut Cursor::new(&data));
        assert_eq!(res, Err(Error::UnexpectedEof));
    }

    #[test]
    fn read_complex_player_as_tag() {
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
        let nbt = nbt.as_tag().compound().unwrap().compound("").unwrap();

        assert_eq!(nbt.float("foodExhaustionLevel").unwrap() as u32, 2);
        assert_eq!(nbt.list("Rotation").unwrap().floats().unwrap().len(), 2);
    }

    #[test]
    fn get_byte_array() {
        // found from fuzzing
        let data = [10, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0];
        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
        nbt.as_compound().to_owned();
    }
    #[test]
    fn list_of_empty_lists() {
        // found from fuzzing
        // BaseNbt { name: m"", tag: NbtTag::NbtCompound { m"":
        // NbtTag::List(List::List([List::Empty])) } }
        let data = [10, 0, 0, 9, 0, 0, 9, 0, 0, 0, 1, 0, 9, 0, 0, 0, 0];
        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
        nbt.as_compound().to_owned();
    }
    #[test]
    fn list_of_byte_arrays() {
        // BaseNbt { name: m"", tag: NbtCompound { values: [(m"",
        // List(List([List::ByteArray([])])))] } }
        let data = [10, 0, 0, 9, 0, 0, 9, 0, 0, 0, 1, 7, 0, 0, 0, 0, 0];
        let nbt = super::read(&mut Cursor::new(&data)).unwrap().unwrap();
        nbt.as_compound().to_owned();
    }

    #[test]
    fn compound_len() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt = super::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap();

        assert_eq!(nbt.as_compound().len(), nbt.as_compound().iter().count());
    }
}
