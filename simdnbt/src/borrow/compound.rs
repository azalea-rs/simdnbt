use std::{io::Cursor, mem::MaybeUninit};

use byteorder::ReadBytesExt;

use crate::{
    common::{
        read_string, skip_string, unchecked_extend, unchecked_push, unchecked_write_string,
        write_string, END_ID, MAX_DEPTH,
    },
    Error, Mutf8Str,
};

use super::{
    extra_tapes::ExtraTapes,
    list::NbtList,
    tape::{MainTape, TapeElement, TapeTagKind, TapeTagValue, UnalignedU16},
    NbtTag, Tapes,
};

#[derive(Debug)]
pub struct NbtCompound<'a: 'tape, 'tape> {
    pub(crate) element: *const TapeElement, // includes the initial compound element
    pub(crate) extra_tapes: &'tape ExtraTapes<'a>,
}

impl<'a: 'tape, 'tape> NbtCompound<'a, 'tape> {
    pub(crate) fn read(
        data: &mut Cursor<&'a [u8]>,
        tapes: &'tape mut Tapes<'a>,
    ) -> Result<(), Error> {
        Self::read_with_depth(data, tapes, 0)
    }

    pub(crate) fn read_with_depth(
        data: &mut Cursor<&'a [u8]>,
        tapes: &'tape mut Tapes<'a>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }

        let index_of_compound_element = tapes.main.elements.len();
        tapes.main.elements.push(TapeElement {
            kind: (
                TapeTagKind::Compound,
                TapeTagValue {
                    // this gets overridden later
                    compound: (0.into(), 0.into()),
                },
            ),
        });

        loop {
            let tag_type = match data.read_u8() {
                Ok(tag_type) => tag_type,
                Err(_) => {
                    return Err(Error::UnexpectedEof);
                }
            };
            if tag_type == END_ID {
                break;
            }

            let tag_name_pointer = data.get_ref().as_ptr() as u64 + data.position();
            debug_assert_eq!(tag_name_pointer >> 56, 0);
            if let Err(e) = skip_string(data) {
                return Err(e);
            };
            tapes.main.elements.push(TapeElement {
                name: tag_name_pointer,
            });
            match NbtTag::read_with_type(data, tapes, tag_type, depth) {
                Ok(tag) => tag,
                Err(e) => {
                    return Err(e);
                }
            };
        }

        let index_after_end_element = tapes.main.elements.len();
        unsafe {
            tapes
                .main
                .elements
                .get_unchecked_mut(index_of_compound_element)
                .kind
                .1
                .compound = (
                0.into(),
                ((index_after_end_element - index_of_compound_element) as u32).into(),
            );
        };

        Ok(())
    }

    // pub fn write(&self, data: &mut Vec<u8>) {
    //     for (name, tag) in self.values {
    //         // reserve 4 bytes extra so we can avoid reallocating for small tags
    //         data.reserve(1 + 2 + name.len() + 4);
    //         // SAFETY: We just reserved enough space for the tag ID, the name length, the name, and
    //         // 4 bytes of tag data.
    //         unsafe {
    //             unchecked_push(data, tag.id());
    //             unchecked_write_string(data, name);
    //         }
    //         match tag {
    //             NbtTag::Byte(byte) => unsafe {
    //                 unchecked_push(data, *byte as u8);
    //             },
    //             NbtTag::Short(short) => unsafe {
    //                 unchecked_extend(data, &short.to_be_bytes());
    //             },
    //             NbtTag::Int(int) => unsafe {
    //                 unchecked_extend(data, &int.to_be_bytes());
    //             },
    //             NbtTag::Long(long) => {
    //                 data.extend_from_slice(&long.to_be_bytes());
    //             }
    //             NbtTag::Float(float) => unsafe {
    //                 unchecked_extend(data, &float.to_be_bytes());
    //             },
    //             NbtTag::Double(double) => {
    //                 data.extend_from_slice(&double.to_be_bytes());
    //             }
    //             NbtTag::ByteArray(byte_array) => {
    //                 unsafe {
    //                     unchecked_extend(data, &byte_array.len().to_be_bytes());
    //                 }
    //                 data.extend_from_slice(byte_array);
    //             }
    //             NbtTag::String(string) => {
    //                 write_string(data, string);
    //             }
    //             NbtTag::List(list) => {
    //                 list.write(data);
    //             }
    //             NbtTag::Compound(compound) => {
    //                 compound.write(data);
    //             }
    //             NbtTag::IntArray(int_array) => {
    //                 unsafe {
    //                     unchecked_extend(data, &int_array.len().to_be_bytes());
    //                 }
    //                 data.extend_from_slice(int_array.as_big_endian());
    //             }
    //             NbtTag::LongArray(long_array) => {
    //                 unsafe {
    //                     unchecked_extend(data, &long_array.len().to_be_bytes());
    //                 }
    //                 data.extend_from_slice(long_array.as_big_endian());
    //             }
    //         }
    //     }
    //     data.push(END_ID);
    // }

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
    pub fn byte_array(&self, name: &str) -> Option<&[u8]> {
        self.get(name).and_then(|tag| tag.byte_array())
    }
    pub fn string(&self, name: &str) -> Option<&Mutf8Str> {
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
    fn element(&self) -> (TapeTagKind, TapeTagValue) {
        unsafe { (*self.element).kind }
    }

    pub fn iter(&self) -> CompoundIter<'a, 'tape> {
        let (kind, value) = self.element();
        debug_assert_eq!(kind, TapeTagKind::Compound);

        let max_tape_offset = u32::from(unsafe { value.list_list.1 }) as usize;
        let tape_slice = unsafe {
            std::slice::from_raw_parts((self.element as *const TapeElement).add(1), max_tape_offset)
        };

        CompoundIter {
            current_tape_offset: 0,
            max_tape_offset,
            tape: tape_slice,
            extra_tapes: self.extra_tapes,
        }
    }

    /// Returns the number of tags directly in this compound.
    ///
    /// Note that due to an optimization, this saturates at 2^24. This means if you have a
    /// compound with more than 2^24 items, then this function will just return 2^24 instead of the
    /// correct length. If you absolutely need the correct length, you can always just iterate over
    /// the compound and get the length that way.
    pub fn len(&self) -> usize {
        let (kind, value) = self.element();
        debug_assert_eq!(kind, TapeTagKind::Compound);
        unsafe { u32::from(value.list_list.0) as usize }
    }

    pub fn exact_len(self) -> usize {
        let len = self.len();
        if len < 2usize.pow(24) {
            len
        } else {
            self.iter().count()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn keys(
        &self,
    ) -> std::iter::Map<
        CompoundIter<'a, 'tape>,
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

pub struct CompoundIter<'a: 'tape, 'tape> {
    current_tape_offset: usize,
    max_tape_offset: usize,
    tape: &'tape [TapeElement],
    extra_tapes: &'tape ExtraTapes<'a>,
}
impl<'a: 'tape, 'tape> Iterator for CompoundIter<'a, 'tape> {
    type Item = (&'a Mutf8Str, NbtTag<'a, 'tape>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.current_tape_offset + 1 >= self.max_tape_offset {
                return None;
            }

            let name_length_ptr = unsafe { self.tape[self.current_tape_offset].name };
            let name_length_ptr = name_length_ptr as *const UnalignedU16;
            let name_length = u16::from(unsafe { *name_length_ptr }).swap_bytes();
            let name_pointer = unsafe { name_length_ptr.add(1) as *const u8 };
            let name_slice =
                unsafe { std::slice::from_raw_parts(name_pointer, name_length as usize) };
            let name = Mutf8Str::from_slice(name_slice);

            self.current_tape_offset += 1;

            let element = unsafe { self.tape.as_ptr().add(self.current_tape_offset as usize) };
            let tag = NbtTag {
                element,
                extra_tapes: self.extra_tapes,
            };

            self.current_tape_offset += unsafe { (*element).skip_offset() };

            return Some((name, tag));
        }
    }
}
