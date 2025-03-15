use std::{marker::PhantomData, mem};

use super::{
    compound::{ParsingStack, ParsingStackElement},
    extra_tapes::{ExtraTapeElement, ExtraTapes},
    tape::{TapeElement, TapeTagKind, UnalignedU32},
    NbtCompound, Tapes,
};
use crate::{
    common::{
        read_i8_array, read_int_array, read_long_array, read_string, read_u8_array,
        read_with_u32_length, slice_i8_into_u8, write_string, write_u32, write_with_u32_length,
        BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID,
        LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    error::NonRootError,
    fastvec::{FastVec, FastVecFromVec},
    raw_list::RawList,
    reader::Reader,
    swap_endianness::SwappableNumber,
    Mutf8Str,
};

/// A list of NBT tags of a single type.
#[derive(Clone, Copy, Debug)]
pub struct NbtList<'a: 'tape, 'tape> {
    pub(crate) element: *const TapeElement, // the initial list element
    pub(crate) extra_tapes: &'tape ExtraTapes<'a>,
}
impl<'a, 'tape> NbtList<'a, 'tape> {
    pub(crate) fn read(
        data: &mut Reader<'a>,
        tapes: &mut Tapes<'a>,
        stack: &mut ParsingStack,
    ) -> Result<(), NonRootError> {
        let tag_type = data.read_u8()?;

        let pushing_element = match tag_type {
            END_ID => {
                // the length is unused for this type of lists
                data.skip(4)?;
                TapeElement::new_with_0(TapeTagKind::EmptyList)
            }
            BYTE_ID => {
                let byte_list_ptr = data.cur;
                let _ = read_i8_array(data)?;
                TapeElement::new_with_ptr(TapeTagKind::ByteList, byte_list_ptr)
            }
            SHORT_ID => {
                let short_list_ptr = data.cur;
                read_with_u32_length(data, 2)?;
                TapeElement::new_with_ptr(TapeTagKind::ShortList, short_list_ptr)
            }
            INT_ID => {
                let int_list_ptr = data.cur;
                read_with_u32_length(data, 4)?;
                TapeElement::new_with_ptr(TapeTagKind::IntList, int_list_ptr)
            }
            LONG_ID => {
                let long_list_ptr = data.cur;
                read_with_u32_length(data, 8)?;
                TapeElement::new_with_ptr(TapeTagKind::LongList, long_list_ptr)
            }
            FLOAT_ID => {
                let float_list_ptr = data.cur;
                read_with_u32_length(data, 4)?;
                TapeElement::new_with_ptr(TapeTagKind::FloatList, float_list_ptr)
            }
            DOUBLE_ID => {
                let double_list_ptr = data.cur;
                read_with_u32_length(data, 8)?;
                TapeElement::new_with_ptr(TapeTagKind::DoubleList, double_list_ptr)
            }
            BYTE_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;

                let length = data.read_u32()?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let byte_array = read_u8_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { byte_array });
                }

                TapeElement::new_with_u32(TapeTagKind::ByteArrayList, index_of_element)
            }
            STRING_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;

                let length = data.read_u32()?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let string = read_string(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { string });
                }

                TapeElement::new_with_u32(TapeTagKind::StringList, index_of_element)
            }
            LIST_ID => {
                let length = data.read_u32()?;
                // length estimate + tape index offset to the end of the list
                let index_of_list_element = tapes.main.len();

                stack.push(ParsingStackElement::list_of_lists(
                    index_of_list_element as u32,
                ))?;
                stack.set_list_length(length);
                TapeElement::new_with_approx_len_and_offset(
                    TapeTagKind::ListList,
                    length,
                    // can't know the offset until after
                    0,
                )
            }
            COMPOUND_ID => {
                let length = data.read_u32()?;
                // length estimate + tape index offset to the end of the compound
                let index_of_list_element = tapes.main.len();

                stack.push(ParsingStackElement::list_of_compounds(
                    index_of_list_element as u32,
                ))?;
                stack.set_list_length(length);
                TapeElement::new_with_approx_len_and_offset(
                    TapeTagKind::CompoundList,
                    length,
                    // this gets overwritten after the list is fully read
                    0,
                )
            }
            INT_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                let length = data.read_u32()?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let int_array = read_int_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { int_array });
                }

                TapeElement::new_with_u32(TapeTagKind::IntArrayList, index_of_element)
            }
            LONG_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                let length = data.read_u32()?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let long_array = read_long_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { long_array });
                }

                TapeElement::new_with_u32(TapeTagKind::LongArrayList, index_of_element)
            }
            _ => return Err(NonRootError::unknown_tag_id(tag_type)),
        };

        tapes.main.push(pushing_element);

        Ok(())
    }

    pub fn write(self, data: &mut Vec<u8>) {
        self.write_fastvec(&mut FastVecFromVec::new(data));
    }

    pub(crate) fn write_fastvec(&self, data: &mut FastVec<u8>) {
        let el = self.element();

        data.push(self.id());

        match el.kind() {
            TapeTagKind::EmptyList => {
                data.extend_from_slice(&0u32.to_be_bytes());
            }
            TapeTagKind::ByteList => {
                write_with_u32_length(data, 1, slice_i8_into_u8(self.bytes().unwrap()));
            }
            TapeTagKind::ShortList => {
                write_with_u32_length(
                    data,
                    2,
                    u32_prefixed_list_to_rawlist::<i16>(TapeTagKind::ShortList, self.element)
                        .unwrap()
                        .as_big_endian(),
                );
            }
            TapeTagKind::IntList => {
                write_with_u32_length(
                    data,
                    4,
                    u32_prefixed_list_to_rawlist::<i32>(TapeTagKind::IntList, self.element)
                        .unwrap()
                        .as_big_endian(),
                );
            }
            TapeTagKind::LongList => {
                write_with_u32_length(
                    data,
                    8,
                    u32_prefixed_list_to_rawlist::<i64>(TapeTagKind::LongList, self.element)
                        .unwrap()
                        .as_big_endian(),
                );
            }
            TapeTagKind::FloatList => {
                write_with_u32_length(
                    data,
                    4,
                    u32_prefixed_list_to_rawlist::<f32>(TapeTagKind::FloatList, self.element)
                        .unwrap()
                        .as_big_endian(),
                );
            }
            TapeTagKind::DoubleList => {
                write_with_u32_length(
                    data,
                    8,
                    u32_prefixed_list_to_rawlist::<f64>(TapeTagKind::DoubleList, self.element)
                        .unwrap()
                        .as_big_endian(),
                );
            }
            TapeTagKind::ByteArrayList => {
                let byte_arrays = self.byte_arrays().unwrap();
                for array in byte_arrays.iter() {
                    write_with_u32_length(data, 1, array);
                }
            }
            TapeTagKind::StringList => {
                let strings = self.strings().unwrap();
                for string in strings.iter() {
                    write_string(data, string);
                }
            }
            TapeTagKind::ListList => {
                let lists = self.lists().unwrap();
                for list in lists {
                    list.write_fastvec(data);
                }
            }
            TapeTagKind::CompoundList => {
                let compounds = self.compounds().unwrap();
                write_u32(data, compounds.clone().len() as u32);
                for compound in compounds {
                    compound.write_fastvec(data);
                }
            }
            TapeTagKind::IntArrayList => {
                let int_arrays = self.int_arrays().unwrap();
                for array in int_arrays.iter() {
                    write_with_u32_length(data, 4, array.as_big_endian());
                }
            }
            TapeTagKind::LongArrayList => {
                let long_arrays = self.long_arrays().unwrap();
                for array in long_arrays.iter() {
                    write_with_u32_length(data, 8, array.as_big_endian());
                }
            }
            _ => unreachable!(),
        }
    }

    /// Get the tape element kind and value for this list.
    fn element(&self) -> TapeElement {
        unsafe { *self.element }
    }

    /// Get the numerical ID of the tag type.
    #[inline]
    pub fn id(&self) -> u8 {
        match self.element().kind() {
            TapeTagKind::EmptyList => END_ID,
            TapeTagKind::ByteList => BYTE_ID,
            TapeTagKind::ShortList => SHORT_ID,
            TapeTagKind::IntList => INT_ID,
            TapeTagKind::LongList => LONG_ID,
            TapeTagKind::FloatList => FLOAT_ID,
            TapeTagKind::DoubleList => DOUBLE_ID,
            TapeTagKind::ByteArrayList => BYTE_ARRAY_ID,
            TapeTagKind::StringList => STRING_ID,
            TapeTagKind::ListList => LIST_ID,
            TapeTagKind::CompoundList => COMPOUND_ID,
            TapeTagKind::IntArrayList => INT_ARRAY_ID,
            TapeTagKind::LongArrayList => LONG_ARRAY_ID,
            _ => unreachable!(),
        }
    }

    /// Returns whether the list is specifically a list with the `empty` tag
    /// type. This will return false if the list is any other type (even it
    /// has a length of zero).
    pub fn empty(&self) -> bool {
        self.element().kind() == TapeTagKind::EmptyList
    }

    pub fn bytes(&self) -> Option<&[i8]> {
        let el = self.element();
        if el.kind() != TapeTagKind::ByteList {
            return None;
        }
        let length_ptr = el.ptr::<UnalignedU32>();
        let length = unsafe { u32::from(*length_ptr) };
        #[cfg(target_endian = "little")]
        let length = length.swap_bytes();
        let byte_array =
            unsafe { std::slice::from_raw_parts(length_ptr.add(1) as *const i8, length as usize) };
        Some(byte_array)
    }
    pub fn shorts(&self) -> Option<Vec<i16>> {
        u32_prefixed_list_to_vec(TapeTagKind::ShortList, self.element)
    }
    pub fn ints(&self) -> Option<Vec<i32>> {
        u32_prefixed_list_to_vec(TapeTagKind::IntList, self.element)
    }
    pub fn longs(&self) -> Option<Vec<i64>> {
        u32_prefixed_list_to_vec(TapeTagKind::LongList, self.element)
    }
    pub fn floats(&self) -> Option<Vec<f32>> {
        u32_prefixed_list_to_vec(TapeTagKind::FloatList, self.element)
    }
    pub fn doubles(&self) -> Option<Vec<f64>> {
        u32_prefixed_list_to_vec(TapeTagKind::DoubleList, self.element)
    }
    pub fn byte_arrays(&self) -> Option<&'a [&'a [u8]]> {
        let el = self.element();
        if el.kind() != TapeTagKind::ByteArrayList {
            return None;
        }
        let index_to_extra_tapes = el.u32() as usize;
        let length_ref = &self.extra_tapes.elements[index_to_extra_tapes];
        let length = unsafe { length_ref.length as usize };
        let slice = unsafe {
            std::slice::from_raw_parts(
                self.extra_tapes
                    .elements
                    .as_ptr()
                    .add(index_to_extra_tapes + 1)
                    .cast(),
                length,
            )
        };
        Some(slice)
    }
    pub fn strings(&self) -> Option<&'a [&'a Mutf8Str]> {
        let el = self.element();
        if el.kind() != TapeTagKind::StringList {
            return None;
        }
        let index_to_extra_tapes = el.u32() as usize;
        let length_ref = &self.extra_tapes.elements[index_to_extra_tapes];
        let length = unsafe { length_ref.length as usize };
        let slice = unsafe {
            std::slice::from_raw_parts(
                self.extra_tapes
                    .elements
                    .as_ptr()
                    .add(index_to_extra_tapes + 1)
                    .cast(),
                length,
            )
        };
        Some(slice)
    }
    pub fn lists(&self) -> Option<NbtListList<'a, 'tape>> {
        let el = self.element();
        if el.kind() != TapeTagKind::ListList {
            return None;
        }

        let (approx_length, max_tape_offset) = el.approx_len_and_offset();

        Some(NbtListList {
            iter: NbtListListIter {
                // it's an iterator, it starts at 0
                current_tape_offset: 0,
                max_tape_offset: max_tape_offset as usize,
                approx_length,
                // the first element is the listlist element so we don't include it
                tape: unsafe { self.element.add(1) },
                extra_tapes: self.extra_tapes,
                _phantom: PhantomData,
            },
        })
    }

    pub fn compounds(&self) -> Option<NbtCompoundList<'a, 'tape>> {
        let el = self.element();
        if el.kind() != TapeTagKind::CompoundList {
            return None;
        }

        let (approx_length, max_tape_offset) = el.approx_len_and_offset();
        let max_tape_offset = max_tape_offset as usize;

        let tape_slice =
            unsafe { std::slice::from_raw_parts(self.element.add(1), max_tape_offset) };

        Some(NbtCompoundList {
            iter: NbtCompoundListIter {
                current_tape_offset: 0,
                max_tape_offset,
                approx_length,
                tape: tape_slice,
                extra_tapes: self.extra_tapes,
            },
        })
    }
    pub fn int_arrays(&self) -> Option<&[RawList<i32>]> {
        let el = self.element();
        if el.kind() != TapeTagKind::IntArrayList {
            return None;
        }
        let index_to_extra_tapes = el.u32() as usize;
        let length_ref = &self.extra_tapes.elements[index_to_extra_tapes];
        let length = unsafe { length_ref.length as usize };
        let slice = unsafe {
            std::slice::from_raw_parts(
                self.extra_tapes
                    .elements
                    .as_ptr()
                    .add(index_to_extra_tapes + 1)
                    .cast(),
                length,
            )
        };
        Some(slice)
    }
    pub fn long_arrays(&self) -> Option<&[RawList<i64>]> {
        let el = self.element();
        if el.kind() != TapeTagKind::LongArrayList {
            return None;
        }
        let index_to_extra_tapes = el.u32() as usize;
        let length_ref = &self.extra_tapes.elements[index_to_extra_tapes];
        let length = unsafe { length_ref.length as usize };
        let slice = unsafe {
            std::slice::from_raw_parts(
                self.extra_tapes
                    .elements
                    .as_ptr()
                    .add(index_to_extra_tapes + 1)
                    .cast(),
                length,
            )
        };
        Some(slice)
    }

    pub fn to_owned(&self) -> crate::owned::NbtList {
        let el = self.element();

        match el.kind() {
            TapeTagKind::EmptyList => crate::owned::NbtList::Empty,
            TapeTagKind::ByteList => crate::owned::NbtList::Byte(self.bytes().unwrap().to_vec()),
            TapeTagKind::ShortList => crate::owned::NbtList::Short(self.shorts().unwrap().to_vec()),
            TapeTagKind::IntList => crate::owned::NbtList::Int(self.ints().unwrap().to_vec()),
            TapeTagKind::LongList => crate::owned::NbtList::Long(self.longs().unwrap().to_vec()),
            TapeTagKind::FloatList => crate::owned::NbtList::Float(self.floats().unwrap().to_vec()),
            TapeTagKind::DoubleList => {
                crate::owned::NbtList::Double(self.doubles().unwrap().to_vec())
            }
            TapeTagKind::ByteArrayList => crate::owned::NbtList::ByteArray(
                self.byte_arrays()
                    .unwrap()
                    .iter()
                    .map(|array| array.to_vec())
                    .collect(),
            ),
            TapeTagKind::StringList => crate::owned::NbtList::String(
                self.strings()
                    .unwrap()
                    .iter()
                    .map(|&string| string.to_owned())
                    .collect(),
            ),
            TapeTagKind::ListList => crate::owned::NbtList::List(
                self.lists()
                    .unwrap()
                    .into_iter()
                    .map(|list| list.to_owned())
                    .collect(),
            ),
            TapeTagKind::CompoundList => crate::owned::NbtList::Compound(
                self.compounds()
                    .unwrap()
                    .into_iter()
                    .map(|compound| compound.to_owned())
                    .collect(),
            ),
            TapeTagKind::IntArrayList => crate::owned::NbtList::IntArray(
                self.int_arrays()
                    .unwrap()
                    .iter()
                    .map(|array| array.to_vec())
                    .collect::<Vec<_>>(),
            ),
            TapeTagKind::LongArrayList => crate::owned::NbtList::LongArray(
                self.long_arrays()
                    .unwrap()
                    .iter()
                    .map(|array| array.to_vec())
                    .collect::<Vec<_>>(),
            ),
            _ => unreachable!("this is an NbtList, no other kinds should be possible"),
        }
    }
}

impl PartialEq for NbtList<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        let self_el = self.element();
        let other_el = other.element();
        if self_el.kind() != other_el.kind() {
            return false;
        }
        match self_el.kind() {
            TapeTagKind::EmptyList => true,
            TapeTagKind::ByteList => self.bytes().unwrap() == other.bytes().unwrap(),
            TapeTagKind::ShortList => self.shorts().unwrap() == other.shorts().unwrap(),
            TapeTagKind::IntList => self.ints().unwrap() == other.ints().unwrap(),
            TapeTagKind::LongList => self.longs().unwrap() == other.longs().unwrap(),
            TapeTagKind::FloatList => self.floats().unwrap() == other.floats().unwrap(),
            TapeTagKind::DoubleList => self.doubles().unwrap() == other.doubles().unwrap(),
            TapeTagKind::ByteArrayList => {
                self.byte_arrays().unwrap() == other.byte_arrays().unwrap()
            }
            TapeTagKind::StringList => self.strings().unwrap() == other.strings().unwrap(),
            TapeTagKind::ListList => self.lists().unwrap() == other.lists().unwrap(),
            TapeTagKind::CompoundList => self.compounds().unwrap() == other.compounds().unwrap(),
            TapeTagKind::IntArrayList => self.int_arrays().unwrap() == other.int_arrays().unwrap(),
            TapeTagKind::LongArrayList => {
                self.long_arrays().unwrap() == other.long_arrays().unwrap()
            }
            _ => unreachable!("this is an NbtList, no other kinds should be possible"),
        }
    }
}

/// A wrapper over [`NbtListListIter`] that acts more like a Vec.
#[derive(Clone, Default)]
pub struct NbtListList<'a, 'tape> {
    iter: NbtListListIter<'a, 'tape>,
}
impl<'a, 'tape> NbtListList<'a, 'tape> {
    /// Returns the number of tags directly in this list.
    ///
    /// Note that due to an internal optimization, this function runs at `O(n)`
    /// if the list has at least 2^24 items. Use [`Self::approx_len`] if you
    /// want to avoid that.
    pub fn len(&self) -> usize {
        self.iter.len()
    }
    /// A version of [`Self::len`] that saturates at 2^24.
    pub fn approx_len(&self) -> u32 {
        self.iter.approx_len()
    }
    /// Get the element at the given index. This is O(n) where n is index, so if
    /// you'll be calling this more than once you should probably just use
    /// the iterator.
    pub fn get(&self, index: usize) -> Option<NbtList<'a, 'tape>> {
        self.iter.clone().nth(index)
    }
    pub fn first(&self) -> Option<NbtList<'a, 'tape>> {
        self.iter.clone().next()
    }
    pub fn last(&self) -> Option<NbtList<'a, 'tape>> {
        self.iter.clone().last()
    }

    pub fn is_empty(self) -> bool {
        self.approx_len() == 0
    }
}
impl<'a: 'tape, 'tape> IntoIterator for NbtListList<'a, 'tape> {
    type Item = NbtList<'a, 'tape>;
    type IntoIter = NbtListListIter<'a, 'tape>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}
impl PartialEq for NbtListList<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        if self.iter.clone().approx_len() != other.iter.clone().approx_len() {
            return false;
        }
        if self.iter.clone().len() != other.iter.clone().len() {
            return false;
        }
        self.iter
            .clone()
            .zip(other.iter.clone())
            .all(|(a, b)| a == b)
    }
}
/// An iterator over a list of lists.
#[derive(Clone)]
pub struct NbtListListIter<'a, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    approx_length: u32,
    tape: *const TapeElement,
    extra_tapes: *const ExtraTapes<'a>,
    _phantom: PhantomData<&'tape ()>,
}
impl<'a: 'tape, 'tape> NbtListListIter<'a, 'tape> {
    /// Returns the number of tags directly in this list.
    ///
    /// Note that due to an internal optimization, this function runs at `O(n)`
    /// if the list has at least 2^24 items. Use [`Self::approx_len`] if you
    /// want to avoid that.
    pub fn len(&self) -> usize {
        let len = self.approx_len();
        if len < 2u32.pow(24) {
            len as usize
        } else {
            self.clone().count()
        }
    }

    /// A version of [`Self::len`] that saturates at 2^24.
    pub fn approx_len(&self) -> u32 {
        self.approx_length
    }

    pub fn is_empty(self) -> bool {
        self.approx_len() == 0
    }
}
impl<'a: 'tape, 'tape> Iterator for NbtListListIter<'a, 'tape> {
    type Item = NbtList<'a, 'tape>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_tape_offset + 1 >= self.max_tape_offset {
            return None;
        }

        let el_ptr = unsafe { self.tape.add(self.current_tape_offset) };
        let el = unsafe { *el_ptr };
        debug_assert!(el.kind().is_list());

        let offset = if matches!(el.kind(), TapeTagKind::CompoundList | TapeTagKind::ListList) {
            el.u32() as usize
        } else {
            1
        };

        let nbt_list = NbtList {
            element: el_ptr,
            extra_tapes: unsafe { &*self.extra_tapes },
        };

        self.current_tape_offset += offset;
        Some(nbt_list)
    }
}
impl Default for NbtListListIter<'_, '_> {
    fn default() -> Self {
        NbtListListIter {
            current_tape_offset: 0,
            max_tape_offset: 0,
            approx_length: 0,
            tape: std::ptr::null(),
            // this won't ever get dereferenced because .next() will return immediately
            extra_tapes: std::ptr::null(),
            _phantom: PhantomData,
        }
    }
}

/// A wrapper over [`NbtCompoundListIter`] that acts more like a Vec.
#[derive(Clone, Default)]
pub struct NbtCompoundList<'a, 'tape> {
    iter: NbtCompoundListIter<'a, 'tape>,
}
impl<'a, 'tape> NbtCompoundList<'a, 'tape> {
    /// Returns the number of tags directly in this list.
    ///
    /// Note that due to an internal optimization, this function runs at `O(n)`
    /// if the list has at least 2^24 items. Use [`Self::approx_len`] if you
    /// want to avoid that.
    pub fn len(&self) -> usize {
        self.iter.len()
    }
    /// A version of [`Self::len`] that saturates at 2^24.
    pub fn approx_len(&self) -> u32 {
        self.iter.approx_len()
    }
    /// Get the element at the given index. This is `O(n)` where n is index, so
    /// if you'll be calling this more than once you should probably just use
    /// the iterator.
    pub fn get(&self, index: usize) -> Option<NbtCompound<'a, 'tape>> {
        self.iter.clone().nth(index)
    }
    pub fn first(&self) -> Option<NbtCompound<'a, 'tape>> {
        self.iter.clone().next()
    }
    pub fn last(&self) -> Option<NbtCompound<'a, 'tape>> {
        self.iter.clone().last()
    }

    pub fn is_empty(self) -> bool {
        self.approx_len() == 0
    }
}
impl<'a: 'tape, 'tape> IntoIterator for NbtCompoundList<'a, 'tape> {
    type Item = NbtCompound<'a, 'tape>;
    type IntoIter = NbtCompoundListIter<'a, 'tape>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}
impl PartialEq for NbtCompoundList<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        if self.iter.clone().approx_len() != other.iter.clone().approx_len() {
            return false;
        }
        if self.iter.clone().len() != other.iter.clone().len() {
            return false;
        }
        self.iter
            .clone()
            .zip(other.iter.clone())
            .all(|(a, b)| a == b)
    }
}

#[derive(Clone)]
pub struct NbtCompoundListIter<'a, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    approx_length: u32,
    tape: &'tape [TapeElement],
    extra_tapes: *const ExtraTapes<'a>,
}
impl<'a: 'tape, 'tape> NbtCompoundListIter<'a, 'tape> {
    /// Returns the number of tags directly in this list.
    ///
    /// Note that due to an internal optimization, this function runs at `O(n)`
    /// if the list has at least 2^24 items. Use [`Self::approx_len`] if you
    /// want to avoid that.
    pub fn len(&self) -> usize {
        let len = self.approx_len();
        if len < 2u32.pow(24) {
            len as usize
        } else {
            self.clone().count()
        }
    }

    /// A version of [`Self::len`] that saturates at 2^24.
    pub fn approx_len(&self) -> u32 {
        self.approx_length
    }

    pub fn is_empty(self) -> bool {
        self.approx_len() == 0
    }
}
impl<'a: 'tape, 'tape> Iterator for NbtCompoundListIter<'a, 'tape> {
    type Item = NbtCompound<'a, 'tape>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_tape_offset + 1 >= self.max_tape_offset {
            return None;
        }

        let el_ptr = unsafe { self.tape.as_ptr().add(self.current_tape_offset) };
        let el = unsafe { *el_ptr };
        debug_assert_eq!(el.kind(), TapeTagKind::Compound);

        let offset = el.u32() as usize;

        let compound = NbtCompound {
            element: el_ptr,
            extra_tapes: unsafe { &*self.extra_tapes },
        };

        self.current_tape_offset += offset;
        Some(compound)
    }
}
impl Default for NbtCompoundListIter<'_, '_> {
    fn default() -> Self {
        NbtCompoundListIter {
            current_tape_offset: 0,
            max_tape_offset: 0,
            approx_length: 0,
            tape: &[],
            // this won't ever get dereferenced because .next() will return immediately
            extra_tapes: std::ptr::null(),
        }
    }
}

pub(crate) fn u32_prefixed_list_to_rawlist<'a, T>(
    expected_kind: TapeTagKind,
    el_ptr: *const TapeElement,
) -> Option<RawList<'a, T>>
where
    T: Copy + SwappableNumber,
{
    let el = unsafe { *el_ptr };
    if el.kind() != expected_kind {
        return None;
    }

    unsafe { u32_prefixed_list_to_rawlist_unchecked(el.ptr()) }
}

#[inline]
pub(crate) unsafe fn u32_prefixed_list_to_rawlist_unchecked<'a, T>(
    ptr: *const UnalignedU32,
) -> Option<RawList<'a, T>>
where
    T: Copy + SwappableNumber,
{
    // length is always a u32
    let length = unsafe { u32::from(*ptr) };
    #[cfg(target_endian = "little")]
    let length = length.swap_bytes();
    let length_in_bytes = length as usize * mem::size_of::<T>();
    let array_be = unsafe { std::slice::from_raw_parts(ptr.add(1) as *const u8, length_in_bytes) };
    Some(RawList::new(array_be))
}

pub(crate) fn u32_prefixed_list_to_vec<T>(
    expected_kind: TapeTagKind,
    element: *const TapeElement,
) -> Option<Vec<T>>
where
    T: Copy + SwappableNumber,
{
    u32_prefixed_list_to_rawlist(expected_kind, element).map(|rawlist| rawlist.to_vec())
}

#[inline]
pub fn read_list_in_list<'a>(
    data: &mut Reader<'a>,
    tapes: &mut Tapes<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let index_of_list_element = stack.peek().index;

    let remaining = stack.remaining_elements_in_list();

    if remaining == 0 {
        stack.pop();

        let index_after_end_element = tapes.main.len();
        unsafe {
            tapes
                .main
                .get_unchecked_mut(index_of_list_element as usize)
                .set_offset(index_after_end_element as u32 - index_of_list_element);
        };
        return Ok(());
    }

    stack.decrement_list_length();

    NbtList::read(data, tapes, stack)
}

#[inline]
pub(crate) fn read_compound_in_list<'a>(
    data: &mut Reader<'a>,
    tapes: &mut Tapes<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let index_of_list_element = stack.peek().index;

    let remaining = stack.remaining_elements_in_list();

    if remaining == 0 {
        stack.pop();

        let index_after_end_element = tapes.main.len();
        unsafe {
            tapes
                .main
                .get_unchecked_mut(index_of_list_element as usize)
                .set_offset(index_after_end_element as u32 - index_of_list_element);
        };
        return Ok(());
    }

    stack.decrement_list_length();

    NbtCompound::read(data, tapes, stack)
}
