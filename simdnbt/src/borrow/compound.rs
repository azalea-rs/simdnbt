use std::mem::MaybeUninit;

use super::{
    extra_tapes::ExtraTapes,
    list::{self, NbtList},
    tape::{TapeElement, TapeTagKind, UnalignedU16},
    NbtTag, Tapes,
};
use crate::{
    common::{
        extend_unchecked, push_unchecked, read_int_array, read_long_array, read_string,
        read_with_u32_length, write_string, write_string_unchecked, BYTE_ARRAY_ID, BYTE_ID,
        COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID,
        LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    error::NonRootError,
    reader::Reader,
    Mutf8Str,
};

#[derive(Debug, Clone, Copy)]
pub struct NbtCompound<'a: 'tape, 'tape> {
    pub(crate) element: *const TapeElement, // includes the initial compound element
    pub(crate) extra_tapes: &'tape ExtraTapes<'a>,
}

impl<'a: 'tape, 'tape> NbtCompound<'a, 'tape> {
    pub(crate) fn read(
        // compounds have no header so nothing to read
        _data: &mut Reader<'a>,
        tapes: &'tape mut Tapes<'a>,
        stack: &mut ParsingStack,
    ) -> Result<(), NonRootError> {
        let index_of_compound_element = tapes.main.len();

        stack.push(ParsingStackElement::compound(
            index_of_compound_element as u32,
        ))?;
        tapes.main.push(TapeElement::new_with_approx_len_and_offset(
            TapeTagKind::Compound,
            // these get overwritten later
            0,
            0,
        ));

        Ok(())
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        for (name, tag) in self.iter() {
            // reserve 4 bytes extra so we can avoid reallocating for small tags
            data.reserve(1 + 2 + name.len() + 4);
            // SAFETY: We just reserved enough space for the tag ID, the name length, the
            // name, and 4 bytes of tag data.
            unsafe {
                push_unchecked(data, tag.id());
                write_string_unchecked(data, name);
            }

            write_tag(tag, data);
        }
        data.push(END_ID);
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<NbtTag<'a, 'tape>> {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for (key, value) in self.iter() {
            if key == name {
                return Some(value);
            }
        }
        None
    }

    /// Returns whether there is a tag with the given name.
    pub fn contains(&self, name: &str) -> bool {
        let name = Mutf8Str::from_str(name);
        let name = name.as_ref();
        for key in self.keys() {
            if key == name {
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
    pub fn byte_array(&self, name: &str) -> Option<&'a [u8]> {
        self.get(name).and_then(|tag| tag.byte_array())
    }
    pub fn string(&self, name: &str) -> Option<&'a Mutf8Str> {
        self.get(name).and_then(|tag| tag.string())
    }
    pub fn list(&self, name: &str) -> Option<NbtList<'a, 'tape>> {
        self.get(name).and_then(|tag| tag.list())
    }
    pub fn compound(&self, name: &str) -> Option<NbtCompound<'a, 'tape>> {
        self.get(name).and_then(|tag| tag.compound())
    }
    pub fn int_array(&self, name: &str) -> Option<Vec<i32>> {
        self.get(name).and_then(|tag| tag.int_array())
    }
    pub fn long_array(&self, name: &str) -> Option<Vec<i64>> {
        self.get(name).and_then(|tag| tag.long_array())
    }

    /// Get the tape element kind and value for this compound.
    fn element(&self) -> TapeElement {
        unsafe { *self.element }
    }

    pub fn iter(&self) -> NbtCompoundIter<'a, 'tape> {
        let el = self.element();
        debug_assert_eq!(el.kind(), TapeTagKind::Compound);

        let max_tape_offset = el.approx_len_and_offset().1 as usize;
        let tape_slice =
            unsafe { std::slice::from_raw_parts(self.element.add(1), max_tape_offset) };

        NbtCompoundIter {
            current_tape_offset: 0,
            max_tape_offset,
            tape: tape_slice,
            extra_tapes: self.extra_tapes,
        }
    }

    /// Returns the number of tags directly in this compound.
    ///
    /// Note that this function runs at `O(n)` due to not storing the length
    /// directly.
    pub fn len(&self) -> usize {
        self.iter().count()

        // let len = self.approx_len();
        // if len < 2u32.pow(24) {
        //     len as usize
        // } else {
        //     self.iter().count()
        // }
    }

    // /// A version of [`Self::len`] that saturates at 2^24.
    // pub fn approx_len(self) -> u32 {
    //     let el = self.element();
    //     debug_assert_eq!(el.kind(), TapeTagKind::Compound);
    //     el.approx_len_and_offset().0
    // }

    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
    #[allow(clippy::type_complexity)]
    pub fn keys(
        &self,
    ) -> std::iter::Map<
        NbtCompoundIter<'a, 'tape>,
        fn((&'a Mutf8Str, NbtTag<'a, 'tape>)) -> &'a Mutf8Str,
    > {
        self.iter().map(|(k, _)| k)
    }

    pub fn to_owned(&self) -> crate::owned::NbtCompound {
        crate::owned::NbtCompound {
            values: self
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.to_owned()))
                .collect(),
        }
    }
}

impl PartialEq for NbtCompound<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

pub struct NbtCompoundIter<'a: 'tape, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    tape: &'tape [TapeElement],
    extra_tapes: &'tape ExtraTapes<'a>,
}
impl<'a: 'tape, 'tape> Iterator for NbtCompoundIter<'a, 'tape> {
    type Item = (&'a Mutf8Str, NbtTag<'a, 'tape>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_tape_offset + 1 >= self.max_tape_offset {
            return None;
        }

        let name_length_ptr = self.tape[self.current_tape_offset].u64() as *const UnalignedU16;
        let name_length = u16::from(unsafe { *name_length_ptr });
        #[cfg(target_endian = "little")]
        let name_length = name_length.swap_bytes();
        let name_ptr = unsafe { name_length_ptr.add(1) as *const u8 };
        let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_length as usize) };
        let name = Mutf8Str::from_slice(name_slice);

        self.current_tape_offset += 1;

        let element = unsafe { self.tape.as_ptr().add(self.current_tape_offset) };
        let tag = NbtTag {
            element,
            extra_tapes: self.extra_tapes,
        };

        self.current_tape_offset += unsafe { (*element).skip_offset() };

        Some((name, tag))
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ParsingStackElement {
    pub kind: ParsingStackElementKind,
    pub index: u32,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum ParsingStackElementKind {
    Compound,
    ListOfCompounds,
    ListOfLists,
}
impl ParsingStackElement {
    pub fn compound(index_of_compound_element: u32) -> Self {
        Self {
            kind: ParsingStackElementKind::Compound,
            index: index_of_compound_element,
        }
    }
    pub fn list_of_compounds(index_of_list_element: u32) -> Self {
        Self {
            kind: ParsingStackElementKind::ListOfCompounds,
            index: index_of_list_element,
        }
    }
    pub fn list_of_lists(index_of_list_element: u32) -> Self {
        Self {
            kind: ParsingStackElementKind::ListOfLists,
            index: index_of_list_element,
        }
    }
}

pub struct ParsingStack {
    stack: [MaybeUninit<ParsingStackElement>; MAX_DEPTH],
    remaining_elements_in_lists: [u32; MAX_DEPTH],
    depth: usize,
}

impl ParsingStack {
    pub fn new() -> Self {
        Self {
            stack: unsafe { MaybeUninit::uninit().assume_init() },
            remaining_elements_in_lists: [0; MAX_DEPTH],
            depth: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, state: ParsingStackElement) -> Result<(), NonRootError> {
        unsafe { self.stack.get_unchecked_mut(self.depth).write(state) };
        self.depth += 1;

        if self.depth >= MAX_DEPTH {
            Err(NonRootError::max_depth_exceeded())
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn set_list_length(&mut self, length: u32) {
        unsafe {
            *self
                .remaining_elements_in_lists
                .get_unchecked_mut(self.depth - 1) = length;
        };
    }

    #[inline]
    pub fn decrement_list_length(&mut self) {
        unsafe {
            *self
                .remaining_elements_in_lists
                .get_unchecked_mut(self.depth - 1) -= 1;
        };
    }

    #[inline]
    pub fn remaining_elements_in_list(&self) -> u32 {
        unsafe {
            *self
                .remaining_elements_in_lists
                .get_unchecked(self.depth - 1)
        }
    }

    #[inline]
    pub fn pop(&mut self) -> ParsingStackElement {
        self.depth -= 1;
        unsafe { self.stack.get_unchecked(self.depth).assume_init() }
    }

    #[inline]
    pub fn peek(&self) -> ParsingStackElement {
        unsafe { self.stack.get_unchecked(self.depth - 1).assume_init() }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.depth == 0
    }

    #[inline]
    pub fn peek_mut(&mut self) -> &mut ParsingStackElement {
        unsafe {
            self.stack
                .get_unchecked_mut(self.depth - 1)
                .assume_init_mut()
        }
    }
}

#[inline(always)]
pub(crate) fn read_tag<'a>(
    data: &mut Reader<'a>,
    tapes: &mut Tapes<'a>,
    stack: &mut ParsingStack,
    tag_type: u8,
) -> Result<(), NonRootError> {
    let pushing_element = match tag_type {
        BYTE_ID => {
            let byte = data.read_i8()?;
            TapeElement::new_with_u8(TapeTagKind::Byte, byte as u8)
        }
        SHORT_ID => {
            let short = data.read_i16()?;
            TapeElement::new_with_u16(TapeTagKind::Short, short as u16)
        }
        INT_ID => {
            let int = data.read_i32()?;
            TapeElement::new_with_u32(TapeTagKind::Int, int as u32)
        }
        LONG_ID => {
            let long_ptr = data.cur;
            data.skip(8)?;
            TapeElement::new_with_ptr(TapeTagKind::Long, long_ptr)
        }
        FLOAT_ID => {
            let float = data.read_f32()?;
            TapeElement::new_with_u32(TapeTagKind::Float, float.to_bits())
        }
        DOUBLE_ID => {
            let double_ptr = data.cur;
            data.skip(8)?;
            TapeElement::new_with_ptr(TapeTagKind::Double, double_ptr)
        }
        BYTE_ARRAY_ID => {
            let byte_array_ptr = data.cur;
            read_with_u32_length(data, 1)?;
            TapeElement::new_with_ptr(TapeTagKind::ByteArray, byte_array_ptr)
        }
        STRING_ID => {
            let string_ptr = data.cur;

            // assert that the top 8 bits of the pointer are 0 (because we rely on this)
            debug_assert_eq!(string_ptr as u64 >> 56, 0);

            read_string(data)?;

            TapeElement::new_with_ptr(TapeTagKind::String, string_ptr)
        }
        COMPOUND_ID => return NbtCompound::read(data, tapes, stack),
        LIST_ID => return NbtList::read(data, tapes, stack),
        INT_ARRAY_ID => {
            let int_array_ptr = data.cur;
            read_int_array(data)?;
            TapeElement::new_with_ptr(TapeTagKind::IntArray, int_array_ptr)
        }
        LONG_ARRAY_ID => {
            let long_array_ptr = data.cur;
            read_long_array(data)?;
            TapeElement::new_with_ptr(TapeTagKind::LongArray, long_array_ptr)
        }
        _ => return Err(NonRootError::unknown_tag_id(tag_type)),
    };

    tapes.main.push(pushing_element);
    Ok(())
}

#[inline]
pub(crate) fn read_tag_in_compound<'a>(
    data: &mut Reader<'a>,
    tapes: &mut Tapes<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let tag_type = data.read_u8()?;
    if tag_type == END_ID {
        handle_compound_end(tapes, stack);
        return Ok(());
    }

    let tag_name_ptr = data.cur;
    debug_assert_eq!(tag_name_ptr as u64 >> 56, 0);

    // read the string in a more efficient way than just calling read_string

    let mut cur_addr = tag_name_ptr as usize;
    let end_addr = data.end_addr();
    cur_addr += 2;
    if cur_addr > end_addr {
        return Err(NonRootError::unexpected_eof());
    }
    // this actually results in an extra instruction since it sets the data.cur
    // unnecessarily, but for some reason it's faster anyways
    let length = unsafe { data.read_type_unchecked::<u16>() }.to_be();
    let length_in_bytes: usize = length as usize;
    cur_addr += length_in_bytes;
    if cur_addr > end_addr {
        return Err(NonRootError::unexpected_eof());
    }
    data.cur = cur_addr as *const u8;

    // finished reading the string

    tapes.main.push(TapeElement::new(tag_name_ptr as u64));

    read_tag(data, tapes, stack, tag_type)
}

#[inline(always)]
fn handle_compound_end(tapes: &mut Tapes, stack: &mut ParsingStack) {
    let index_of_compound_element = stack.pop().index;
    let index_after_end_element = tapes.main.len();

    unsafe {
        tapes
            .main
            .get_unchecked_mut(index_of_compound_element as usize)
            // we don't set the approx_len because determining it for compounds
            // is too expensive
            .set_offset(index_after_end_element as u32 - index_of_compound_element);
    };
}

pub(crate) fn write_tag(tag: NbtTag, data: &mut Vec<u8>) {
    let el = tag.element();
    match el.kind() {
        TapeTagKind::Byte => unsafe {
            push_unchecked(data, tag.byte().unwrap() as u8);
        },
        TapeTagKind::Short => unsafe {
            extend_unchecked(data, &tag.short().unwrap().to_be_bytes());
        },
        TapeTagKind::Int => unsafe {
            extend_unchecked(data, &tag.int().unwrap().to_be_bytes());
        },
        TapeTagKind::Long => {
            data.extend_from_slice(&tag.long().unwrap().to_be_bytes());
        }
        TapeTagKind::Float => unsafe {
            extend_unchecked(data, &tag.float().unwrap().to_be_bytes());
        },
        TapeTagKind::Double => {
            data.extend_from_slice(&tag.double().unwrap().to_be_bytes());
        }
        TapeTagKind::ByteArray => {
            let byte_array = tag.byte_array().unwrap();
            unsafe {
                extend_unchecked(data, &(byte_array.len() as u32).to_be_bytes());
            }
            data.extend_from_slice(byte_array);
        }
        TapeTagKind::String => {
            let string = tag.string().unwrap();
            write_string(data, string);
        }
        kind if kind.is_list() => {
            tag.list().unwrap().write(data);
        }
        TapeTagKind::Compound => {
            tag.compound().unwrap().write(data);
        }
        TapeTagKind::IntArray => {
            let int_array =
                unsafe { list::u32_prefixed_list_to_rawlist_unchecked::<i32>(el.ptr()).unwrap() };
            unsafe {
                extend_unchecked(data, &(int_array.len() as u32).to_be_bytes());
            }
            data.extend_from_slice(int_array.as_big_endian());
        }
        TapeTagKind::LongArray => {
            let long_array =
                unsafe { list::u32_prefixed_list_to_rawlist_unchecked::<i64>(el.ptr()).unwrap() };
            unsafe {
                extend_unchecked(data, &(long_array.len() as u32).to_be_bytes());
            }
            data.extend_from_slice(long_array.as_big_endian());
        }
        kind => unreachable!("Invalid tag kind {kind:?}"),
    }
}
