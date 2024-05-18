use std::io::Cursor;

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_i8_array, read_int_array, read_long_array, read_string, read_u8_array,
        read_with_u32_length, slice_i8_into_u8, unchecked_extend, unchecked_push, write_string,
        write_u32, write_with_u32_length, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID,
        FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    raw_list::RawList,
    swap_endianness::SwappableNumber,
    Error, Mutf8Str,
};

use super::{
    extra_tapes::{ExtraTapeElement, ExtraTapes},
    read_u32,
    tape::{TapeElement, TapeTagKind, TapeTagValue, UnalignedU32},
    NbtCompound, Tapes, MAX_DEPTH,
};

/// A list of NBT tags of a single type.
pub struct NbtList<'a: 'tape, 'tape> {
    pub(crate) element: *const TapeElement, // the initial list element
    pub(crate) extra_tapes: &'tape ExtraTapes<'a>,
}
impl<'a, 'tape> NbtList<'a, 'tape> {
    pub(crate) fn read(
        data: &mut Cursor<&'a [u8]>,
        tapes: &mut Tapes<'a>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }
        let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        Ok(match tag_type {
            // END_ID => {
            //     data.set_position(data.position() + 4);
            //     NbtList::Empty
            // }
            END_ID => {
                data.set_position(data.position() + 4);
                tapes.main.elements.push(TapeElement {
                    kind: (TapeTagKind::EmptyList, TapeTagValue { empty_list: () }),
                });
            }
            // BYTE_ID => NbtList::Byte(read_i8_array(data)?),
            BYTE_ID => {
                let byte_list_pointer = data.get_ref().as_ptr() as u64 + data.position() as u64;
                let _ = read_i8_array(data)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::ByteList,
                        TapeTagValue {
                            byte_list: byte_list_pointer.into(),
                        },
                    ),
                });
            }
            // SHORT_ID => NbtList::Short(RawList::new(read_with_u32_length(data, 2)?)),
            SHORT_ID => {
                let short_list_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 2)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::ShortList,
                        TapeTagValue {
                            short_list: short_list_pointer.into(),
                        },
                    ),
                });
            }
            // INT_ID => NbtList::Int(RawList::new(read_with_u32_length(data, 4)?)),
            INT_ID => {
                let int_list_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 4)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::IntList,
                        TapeTagValue {
                            int_list: int_list_pointer.into(),
                        },
                    ),
                });
            }
            // LONG_ID => NbtList::Long(RawList::new(read_with_u32_length(data, 8)?)),
            LONG_ID => {
                let long_list_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 8)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::LongList,
                        TapeTagValue {
                            long_list: long_list_pointer.into(),
                        },
                    ),
                });
            }
            // FLOAT_ID => NbtList::Float(RawList::new(read_with_u32_length(data, 4)?)),
            FLOAT_ID => {
                let float_list_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 4)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::FloatList,
                        TapeTagValue {
                            float_list: float_list_pointer.into(),
                        },
                    ),
                });
            }
            // DOUBLE_ID => NbtList::Double(RawList::new(read_with_u32_length(data, 8)?)),
            DOUBLE_ID => {
                let double_list_pointer = data.get_ref().as_ptr() as u64 + data.position();
                read_with_u32_length(data, 8)?;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::DoubleList,
                        TapeTagValue {
                            double_list: double_list_pointer.into(),
                        },
                    ),
                });
            }
            // BYTE_ARRAY_ID => NbtList::ByteArray({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_bytearray.start();
            //     for _ in 0..length {
            //         let tag = match read_u8_array(data) {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_bytearray.finish(tags);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag);
            //     }
            //     alloc.get().unnamed_bytearray.finish(tags)
            // }),
            BYTE_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::ByteArrayList,
                        TapeTagValue {
                            byte_array_list: (0.into(), index_of_element.into()),
                        },
                    ),
                });

                let length = read_u32(data)?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let byte_array = read_u8_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { byte_array });
                }
            }
            // STRING_ID => NbtList::String({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_string.start();
            //     for _ in 0..length {
            //         let tag = match read_string(data) {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_string.finish(tags);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag);
            //     }
            //     alloc.get().unnamed_string.finish(tags)
            // }),
            STRING_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::StringList,
                        TapeTagValue {
                            string_list: (0.into(), index_of_element.into()),
                        },
                    ),
                });

                let length = read_u32(data)?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let string = read_string(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { string });
                }
            }
            // LIST_ID => NbtList::List({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_list.start(list_depth);
            //     for _ in 0..length {
            //         let tag = match NbtList::read(data, alloc, compound_depth, list_depth + 1) {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_list.finish(tags, list_depth);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag)
            //     }
            //     alloc.get().unnamed_list.finish(tags, list_depth)
            // }),
            LIST_ID => {
                let length = read_u32(data)?;
                // length estimate + tape index offset to the end of the list
                let index_of_list_element = tapes.main.elements.len();
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::ListList,
                        TapeTagValue {
                            // can't know the offset until after
                            list_list: (length.into(), 0.into()),
                        },
                    ),
                });
                for _ in 0..length {
                    NbtList::read(data, tapes, depth + 1)?;
                }
                let index_after_end_element = tapes.main.elements.len();
                unsafe {
                    tapes
                        .main
                        .elements
                        .get_unchecked_mut(index_of_list_element)
                        .kind
                        .1
                        .list_list = (
                        0.into(),
                        ((index_after_end_element - index_of_list_element) as u32).into(),
                    )
                };
            }
            // COMPOUND_ID => NbtList::Compound({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_compound.start(list_depth);
            //     for _ in 0..length {
            //         let tag_res = unsafe {
            //             NbtCompound::read_with_depth(data, alloc, compound_depth + 1, list_depth)
            //         };
            //         let tag = match tag_res {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_compound.finish(tags, list_depth);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag);
            //     }
            //     alloc.get().unnamed_compound.finish(tags, list_depth)
            // }),
            COMPOUND_ID => {
                let length = read_u32(data)?;
                // length estimate + tape index offset to the end of the compound
                let index_of_compound_element = tapes.main.elements.len();
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::CompoundList,
                        TapeTagValue {
                            compound_list: (length.into(), 0.into()),
                        },
                    ),
                });
                for _ in 0..length {
                    NbtCompound::read_with_depth(data, tapes, depth + 1)?;
                }
                let index_after_end_element = tapes.main.elements.len();
                unsafe {
                    tapes
                        .main
                        .elements
                        .get_unchecked_mut(index_of_compound_element)
                        .kind
                        .1
                        .compound_list = (
                        0.into(),
                        ((index_after_end_element - index_of_compound_element) as u32).into(),
                    )
                };
            }
            // INT_ARRAY_ID => NbtList::IntArray({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_intarray.start();
            //     for _ in 0..length {
            //         let tag = match read_int_array(data) {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_intarray.finish(tags);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag);
            //     }
            //     alloc.get().unnamed_intarray.finish(tags)
            // }),
            INT_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::IntArrayList,
                        TapeTagValue {
                            int_array_list: (0.into(), index_of_element.into()),
                        },
                    ),
                });
                let length = read_u32(data)?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let int_array = read_int_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { int_array });
                }
            }
            // LONG_ARRAY_ID => NbtList::LongArray({
            //     let length = read_u32(data)?;
            //     let mut tags = alloc.get().unnamed_longarray.start();
            //     for _ in 0..length {
            //         let tag = match read_long_array(data) {
            //             Ok(tag) => tag,
            //             Err(e) => {
            //                 alloc.get().unnamed_longarray.finish(tags);
            //                 return Err(e);
            //             }
            //         };
            //         tags.push(tag);
            //     }
            //     alloc.get().unnamed_longarray.finish(tags)
            // }),
            LONG_ARRAY_ID => {
                let index_of_element = tapes.extra.elements.len() as u32;
                tapes.main.elements.push(TapeElement {
                    kind: (
                        TapeTagKind::LongArrayList,
                        TapeTagValue {
                            long_array_list: (0.into(), index_of_element.into()),
                        },
                    ),
                });
                let length = read_u32(data)?;
                tapes.extra.elements.push(ExtraTapeElement { length });
                for _ in 0..length {
                    let long_array = read_long_array(data)?;
                    tapes.extra.elements.push(ExtraTapeElement { long_array });
                }
            }
            _ => return Err(Error::UnknownTagId(tag_type)),
        })
    }

    // pub fn write(&self, data: &mut Vec<u8>) {
    //     // fast path for compound since it's very common to have lists of compounds
    //     if let NbtList::Compound(compounds) = self {
    //         data.reserve(5);
    //         // SAFETY: we just reserved 5 bytes
    //         unsafe {
    //             unchecked_push(data, COMPOUND_ID);
    //             unchecked_extend(data, &(compounds.len() as u32).to_be_bytes());
    //         }
    //         for compound in *compounds {
    //             compound.write(data);
    //         }
    //         return;
    //     }

    //     data.push(self.id());
    //     match self {
    //         NbtList::Empty => {
    //             data.extend(&0u32.to_be_bytes());
    //         }
    //         NbtList::Byte(bytes) => {
    //             write_with_u32_length(data, 1, slice_i8_into_u8(bytes));
    //         }
    //         NbtList::Short(shorts) => {
    //             write_with_u32_length(data, 2, shorts.as_big_endian());
    //         }
    //         NbtList::Int(ints) => {
    //             write_with_u32_length(data, 4, ints.as_big_endian());
    //         }
    //         NbtList::Long(longs) => {
    //             write_with_u32_length(data, 8, longs.as_big_endian());
    //         }
    //         NbtList::Float(floats) => {
    //             write_with_u32_length(data, 4, floats.as_big_endian());
    //         }
    //         NbtList::Double(doubles) => {
    //             write_with_u32_length(data, 8, doubles.as_big_endian());
    //         }
    //         NbtList::ByteArray(byte_arrays) => {
    //             write_u32(data, byte_arrays.len() as u32);
    //             for array in byte_arrays.iter() {
    //                 write_with_u32_length(data, 1, array);
    //             }
    //         }
    //         NbtList::String(strings) => {
    //             write_u32(data, strings.len() as u32);
    //             for string in *strings {
    //                 write_string(data, string);
    //             }
    //         }
    //         NbtList::List(lists) => {
    //             write_u32(data, lists.len() as u32);
    //             for list in *lists {
    //                 list.write(data);
    //             }
    //         }
    //         NbtList::Compound(_) => {
    //             unreachable!("fast path for compound should have been taken")
    //         }
    //         NbtList::IntArray(int_arrays) => {
    //             write_u32(data, int_arrays.len() as u32);
    //             for array in *int_arrays {
    //                 write_with_u32_length(data, 4, array.as_big_endian());
    //             }
    //         }
    //         NbtList::LongArray(long_arrays) => {
    //             write_u32(data, long_arrays.len() as u32);
    //             for array in *long_arrays {
    //                 write_with_u32_length(data, 8, array.as_big_endian());
    //             }
    //         }
    //     }
    // }

    /// Get the tape element kind and value for this list.
    fn element(&self) -> (TapeTagKind, TapeTagValue) {
        unsafe { (*self.element).kind }
    }

    /// Get the numerical ID of the tag type.
    #[inline]
    pub fn id(&self) -> u8 {
        match self.element().0 {
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

    pub fn bytes(&self) -> Option<&[i8]> {
        // match self {
        //     NbtList::Byte(bytes) => Some(bytes),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::ByteList {
            return None;
        }
        let length_ptr = u64::from(unsafe { value.byte_list }) as usize as *const UnalignedU32;
        let length = unsafe { u32::from(*length_ptr).swap_bytes() as usize };
        let byte_array =
            unsafe { std::slice::from_raw_parts(length_ptr.add(1) as *const i8, length) };
        Some(byte_array)
    }
    pub fn shorts(&self) -> Option<Vec<i16>> {
        // match self {
        //     NbtList::Short(shorts) => Some(shorts.to_vec()),
        //     _ => None,
        // }
        u32_prefixed_list_to_vec(TapeTagKind::ShortList, self.element)
    }
    pub fn ints(&self) -> Option<Vec<i32>> {
        // match self {
        //     NbtList::Int(ints) => Some(ints.to_vec()),
        //     _ => None,
        // }
        u32_prefixed_list_to_vec(TapeTagKind::IntList, self.element)
    }
    pub fn longs(&self) -> Option<Vec<i64>> {
        // match self {
        //     NbtList::Long(longs) => Some(longs.to_vec()),
        //     _ => None,
        // }
        u32_prefixed_list_to_vec(TapeTagKind::LongList, self.element)
    }
    pub fn floats(&self) -> Option<Vec<f32>> {
        // match self {
        //     NbtList::Float(floats) => Some(floats.to_vec()),
        //     _ => None,
        // }
        u32_prefixed_list_to_vec(TapeTagKind::FloatList, self.element)
    }
    pub fn doubles(&self) -> Option<Vec<f64>> {
        // match self {
        //     NbtList::Double(doubles) => Some(doubles.to_vec()),
        //     _ => None,
        // }
        u32_prefixed_list_to_vec(TapeTagKind::DoubleList, self.element)
    }
    pub fn byte_arrays(&self) -> Option<&[&[u8]]> {
        // match self {
        //     NbtList::ByteArray(byte_arrays) => Some(byte_arrays),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::ByteArrayList {
            return None;
        }
        let index_to_extra_tapes = u32::from(unsafe { value.byte_array_list.1 }) as usize;
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
    pub fn strings(&self) -> Option<&[&Mutf8Str]> {
        // match self {
        //     NbtList::String(strings) => Some(strings),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::StringList {
            return None;
        }
        let index_to_extra_tapes = u32::from(unsafe { value.string_list.1 }) as usize;
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
    pub fn lists(&self) -> Option<ListListIter> {
        // match self {
        //     NbtList::List(lists) => Some(lists),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::ListList {
            return None;
        }

        let max_tape_offset = u32::from(unsafe { value.list_list.1 }) as usize;
        let tape_slice = unsafe {
            std::slice::from_raw_parts((self.element as *const TapeElement).add(1), max_tape_offset)
        };

        Some(ListListIter {
            current_tape_offset: 0, // it's an iterator, it starts at 0
            max_tape_offset,
            tape: tape_slice, // the first element is the listlist element so we don't include it
            extra_tapes: self.extra_tapes,
        })
    }

    pub fn compounds(&self) -> Option<CompoundListIter> {
        // match self {
        //     NbtList::Compound(compounds) => Some(compounds),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::CompoundList {
            return None;
        }

        let length = u32::from(unsafe { value.compound_list.0 }) as usize;

        let max_tape_offset = u32::from(unsafe { value.compound_list.1 }) as usize;
        let tape_slice = unsafe {
            std::slice::from_raw_parts((self.element as *const TapeElement).add(1), max_tape_offset)
        };

        Some(CompoundListIter {
            current_tape_offset: 0,
            max_tape_offset,
            length,
            tape: tape_slice,
            extra_tapes: self.extra_tapes,
        })
    }
    pub fn int_arrays(&self) -> Option<&[RawList<i32>]> {
        // match self {
        //     NbtList::IntArray(int_arrays) => Some(int_arrays),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::IntArrayList {
            return None;
        }
        let index_to_extra_tapes = u32::from(unsafe { value.int_array_list.1 }) as usize;
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
        // match self {
        //     NbtList::LongArray(long_arrays) => Some(long_arrays),
        //     _ => None,
        // }
        let (kind, value) = self.element();
        if kind != TapeTagKind::LongArrayList {
            return None;
        }
        let index_to_extra_tapes = u32::from(unsafe { value.long_array_list.1 }) as usize;
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
        todo!();
        // match self {
        //     NbtList::Empty => crate::owned::NbtList::Empty,
        //     NbtList::Byte(bytes) => crate::owned::NbtList::Byte(bytes.to_vec()),
        //     NbtList::Short(shorts) => crate::owned::NbtList::Short(shorts.to_vec()),
        //     NbtList::Int(ints) => crate::owned::NbtList::Int(ints.to_vec()),
        //     NbtList::Long(longs) => crate::owned::NbtList::Long(longs.to_vec()),
        //     NbtList::Float(floats) => crate::owned::NbtList::Float(floats.to_vec()),
        //     NbtList::Double(doubles) => crate::owned::NbtList::Double(doubles.to_vec()),
        //     NbtList::ByteArray(byte_arrays) => crate::owned::NbtList::ByteArray(
        //         byte_arrays.iter().map(|array| array.to_vec()).collect(),
        //     ),
        //     NbtList::String(strings) => crate::owned::NbtList::String(
        //         strings.iter().map(|&string| string.to_owned()).collect(),
        //     ),
        //     NbtList::List(lists) => {
        //         crate::owned::NbtList::List(lists.iter().map(|list| list.to_owned()).collect())
        //     }
        //     NbtList::Compound(compounds) => crate::owned::NbtList::Compound(
        //         compounds
        //             .iter()
        //             .map(|compound| compound.to_owned())
        //             .collect(),
        //     ),
        //     NbtList::IntArray(int_arrays) => crate::owned::NbtList::IntArray(
        //         int_arrays
        //             .iter()
        //             .map(|array| array.to_vec())
        //             .collect::<Vec<_>>(),
        //     ),
        //     NbtList::LongArray(long_arrays) => crate::owned::NbtList::LongArray(
        //         long_arrays
        //             .iter()
        //             .map(|array| array.to_vec())
        //             .collect::<Vec<_>>(),
        //     ),
        // }
    }

    pub fn as_nbt_tags(&self) -> Vec<super::NbtTag> {
        todo!();
        // match self {
        //     NbtList::Empty => vec![],
        //     NbtList::Byte(bytes) => bytes
        //         .iter()
        //         .map(|&byte| super::NbtTag::Byte(byte))
        //         .collect(),
        //     NbtList::Short(shorts) => shorts
        //         .to_vec()
        //         .into_iter()
        //         .map(super::NbtTag::Short)
        //         .collect(),
        //     NbtList::Int(ints) => ints.to_vec().into_iter().map(super::NbtTag::Int).collect(),
        //     NbtList::Long(longs) => longs
        //         .to_vec()
        //         .into_iter()
        //         .map(super::NbtTag::Long)
        //         .collect(),
        //     NbtList::Float(floats) => floats
        //         .to_vec()
        //         .into_iter()
        //         .map(super::NbtTag::Float)
        //         .collect(),
        //     NbtList::Double(doubles) => doubles
        //         .to_vec()
        //         .into_iter()
        //         .map(super::NbtTag::Double)
        //         .collect(),
        //     NbtList::ByteArray(byte_arrays) => byte_arrays
        //         .iter()
        //         .map(|&array| super::NbtTag::ByteArray(array))
        //         .collect(),
        //     NbtList::String(strings) => strings
        //         .iter()
        //         .map(|&string| super::NbtTag::String(string))
        //         .collect(),
        //     NbtList::List(lists) => lists
        //         .iter()
        //         .map(|list| super::NbtTag::List(list.clone()))
        //         .collect(),
        //     NbtList::Compound(compounds) => compounds
        //         .iter()
        //         .map(|compound| super::NbtTag::Compound(compound.clone()))
        //         .collect(),
        //     NbtList::IntArray(int_arrays) => int_arrays
        //         .iter()
        //         .map(|array| super::NbtTag::IntArray(array.clone()))
        //         .collect(),
        //     NbtList::LongArray(long_arrays) => long_arrays
        //         .iter()
        //         .map(|array| super::NbtTag::LongArray(array.clone()))
        //         .collect(),
        // }
    }
}

/// An iterator over a list of lists.
pub struct ListListIter<'a, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    tape: &'tape [TapeElement],
    extra_tapes: &'tape ExtraTapes<'a>,
}
impl<'a: 'tape, 'tape> Iterator for ListListIter<'a, 'tape> {
    type Item = NbtList<'a, 'tape>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.current_tape_offset + 1 >= self.max_tape_offset {
                return None;
            }
            let element = &self.tape[self.current_tape_offset as usize];
            let (kind, value) = unsafe { element.kind };
            debug_assert!(kind.is_list());

            let offset = u32::from(unsafe { value.list_list.1 }) as usize;

            let nbt_list = NbtList {
                element,
                extra_tapes: self.extra_tapes,
            };

            self.current_tape_offset += offset;
            return Some(nbt_list);
        }
    }
}
pub struct CompoundListIter<'a, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    length: usize,
    tape: &'tape [TapeElement],
    extra_tapes: *const ExtraTapes<'a>,
}
impl<'a: 'tape, 'tape> CompoundListIter<'a, 'tape> {
    /// Returns the number of tags directly in this list.
    ///
    /// Note that due to an optimization, this saturates at 2^24. Use [`Self::exact_len`] if you
    /// need the length to always be accurate at extremes.
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn exact_len(self) -> usize {
        let len = self.len();
        if len < 2usize.pow(24) {
            len
        } else {
            self.count()
        }
    }
}
impl<'a: 'tape, 'tape> Iterator for CompoundListIter<'a, 'tape> {
    type Item = NbtCompound<'a, 'tape>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.current_tape_offset + 1 >= self.max_tape_offset {
                return None;
            }

            let element = &self.tape[self.current_tape_offset as usize];
            let (kind, value) = unsafe { element.kind };
            debug_assert_eq!(kind, TapeTagKind::Compound);

            let offset = u32::from(unsafe { value.compound_list.1 }) as usize;

            let compound = NbtCompound {
                element,
                extra_tapes: unsafe { &*self.extra_tapes },
            };

            self.current_tape_offset += offset;
            return Some(compound);
        }
    }
}
impl Default for CompoundListIter<'_, '_> {
    fn default() -> Self {
        CompoundListIter {
            current_tape_offset: 0,
            max_tape_offset: 0,
            length: 0,
            tape: &[],
            // this won't ever get dereferenced because .next() will return immediately
            extra_tapes: std::ptr::null(),
        }
    }
}

pub(crate) fn u32_prefixed_list_to_vec<T>(
    expected_kind: TapeTagKind,
    element: *const TapeElement,
) -> Option<Vec<T>>
where
    T: Copy + SwappableNumber,
{
    let (kind, value) = unsafe { (*element).kind };
    if kind != expected_kind {
        return None;
    }
    // length is always a u32
    let length_ptr = u64::from(unsafe { value.int_list }) as usize as *const UnalignedU32;
    let length = unsafe { u32::from(*length_ptr).swap_bytes() as usize };
    let length_in_bytes = length * std::mem::size_of::<T>();
    let array_be =
        unsafe { std::slice::from_raw_parts(length_ptr.add(1) as *const u8, length_in_bytes) };
    Some(RawList::new(array_be).to_vec())
}
