use super::{
    compound::{ParsingStack, ParsingStackElementKind},
    NbtCompound,
};
use crate::{
    common::{
        read_i8_array, read_int_array, read_long_array, read_string, read_u8_array,
        read_with_u32_length, BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, END_ID, FLOAT_ID,
        INT_ARRAY_ID, INT_ID, LIST_ID, LONG_ARRAY_ID, LONG_ID, SHORT_ID, STRING_ID,
    },
    error::NonRootError,
    reader::Reader,
};

/// A list of NBT tags of a single type.
#[derive(Clone, Copy, Debug)]
pub struct NbtList;
impl NbtList {
    pub(crate) fn read(data: &mut Reader, stack: &mut ParsingStack) -> Result<(), NonRootError> {
        let tag_type = data.read_u8()?;

        match tag_type {
            END_ID => {
                // the length is unused for this type of lists
                data.skip(4)?;
            }
            BYTE_ID => {
                let _ = read_i8_array(data)?;
            }
            SHORT_ID => {
                read_with_u32_length(data, 2)?;
            }
            INT_ID => {
                read_with_u32_length(data, 4)?;
            }
            LONG_ID => {
                read_with_u32_length(data, 8)?;
            }
            FLOAT_ID => {
                read_with_u32_length(data, 4)?;
            }
            DOUBLE_ID => {
                read_with_u32_length(data, 8)?;
            }
            BYTE_ARRAY_ID => {
                let length = data.read_u32()?;
                for _ in 0..length {
                    let _ = read_u8_array(data)?;
                }
            }
            STRING_ID => {
                let length = data.read_u32()?;
                for _ in 0..length {
                    let _ = read_string(data)?;
                }
            }
            LIST_ID => {
                let length = data.read_u32()?;
                // length estimate + tape index offset to the end of the list

                stack.push(ParsingStackElementKind::ListOfLists)?;
                stack.set_list_length(length);
            }
            COMPOUND_ID => {
                let length = data.read_u32()?;

                stack.push(ParsingStackElementKind::ListOfCompounds)?;
                stack.set_list_length(length);
            }
            INT_ARRAY_ID => {
                let length = data.read_u32()?;
                for _ in 0..length {
                    let _ = read_int_array(data)?;
                }
            }
            LONG_ARRAY_ID => {
                let length = data.read_u32()?;
                for _ in 0..length {
                    let _ = read_long_array(data)?;
                }
            }
            _ => return Err(NonRootError::unknown_tag_id(tag_type)),
        };

        Ok(())
    }
}

#[inline]
pub fn read_list_in_list<'a>(
    data: &mut Reader<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let remaining = stack.remaining_elements_in_list();
    if remaining == 0 {
        stack.pop();
        return Ok(());
    }
    stack.decrement_list_length();
    NbtList::read(data, stack)
}

#[inline]
pub(crate) fn read_compound_in_list<'a>(
    data: &mut Reader<'a>,
    stack: &mut ParsingStack,
) -> Result<(), NonRootError> {
    let remaining = stack.remaining_elements_in_list();
    if remaining == 0 {
        stack.pop();
        return Ok(());
    }
    stack.decrement_list_length();
    NbtCompound::read(data, stack)
}
