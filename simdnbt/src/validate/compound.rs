use std::mem::MaybeUninit;

use super::list::NbtList;
use crate::{
    common::{
        read_int_array, read_long_array, read_string, read_with_u32_length, BYTE_ARRAY_ID, BYTE_ID,
        COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID,
        LONG_ID, MAX_DEPTH, SHORT_ID, STRING_ID,
    },
    error::NonRootError,
    reader::Reader,
};

#[derive(Debug, Clone, Copy)]
pub struct NbtCompound;

impl NbtCompound {
    pub(crate) fn read(
        // compounds have no header so nothing to read
        _data: &mut Reader,
        stack: &mut ParsingStack,
    ) -> Result<(), NonRootError> {
        stack.push(ParsingStackElementKind::Compound)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum ParsingStackElementKind {
    Compound,
    ListOfCompounds,
    ListOfLists,
}

pub struct ParsingStack {
    stack: [MaybeUninit<ParsingStackElementKind>; MAX_DEPTH],
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
    pub fn push(&mut self, state: ParsingStackElementKind) -> Result<(), NonRootError> {
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
    pub fn pop(&mut self) -> ParsingStackElementKind {
        self.depth -= 1;
        unsafe { self.stack.get_unchecked(self.depth).assume_init() }
    }

    #[inline]
    pub fn peek(&self) -> ParsingStackElementKind {
        unsafe { self.stack.get_unchecked(self.depth - 1).assume_init() }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.depth == 0
    }
}

#[inline(always)]
pub fn read_tag<'a>(
    data: &mut Reader<'a>,
    stack: &mut ParsingStack,
    tag_type: u8,
) -> Result<(), NonRootError> {
    match tag_type {
        BYTE_ID => {
            let _ = data.read_i8()?;
        }
        SHORT_ID => {
            let _ = data.read_i16()?;
        }
        INT_ID => {
            let _ = data.read_i32()?;
        }
        LONG_ID => {
            data.skip(8)?;
        }
        FLOAT_ID => {
            let _ = data.read_f32()?;
        }
        DOUBLE_ID => {
            data.skip(8)?;
        }
        BYTE_ARRAY_ID => {
            read_with_u32_length(data, 1)?;
        }
        STRING_ID => {
            read_string(data)?;
        }
        COMPOUND_ID => return NbtCompound::read(data, stack),
        LIST_ID => return NbtList::read(data, stack),
        INT_ARRAY_ID => {
            read_int_array(data)?;
        }
        LONG_ARRAY_ID => {
            read_long_array(data)?;
        }
        _ => return Err(NonRootError::unknown_tag_id(tag_type)),
    };

    Ok(())
}

#[inline]
pub(crate) fn read_tag_in_compound<'a>(
    data: &mut Reader<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let tag_type = data.read_u8()?;
    if tag_type == END_ID {
        handle_compound_end(stack);
        return Ok(());
    }

    let tag_name_ptr = data.cur;
    debug_assert_eq!(tag_name_ptr as u64 >> 56, 0);

    // read the tag name in a more efficient way than just calling read_string

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

    // finished reading the tag name

    read_tag(data, stack, tag_type)
}

#[inline(always)]
fn handle_compound_end(stack: &mut ParsingStack) {
    stack.pop();
}
