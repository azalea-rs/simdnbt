use std::{io::Cursor, slice};

use crate::{
    raw_list::RawList,
    swap_endianness::{swap_endianness_as_u8, SwappableNumber},
    Mutf8Str, ReadError,
};

pub const END_ID: u8 = 0;
pub const BYTE_ID: u8 = 1;
pub const SHORT_ID: u8 = 2;
pub const INT_ID: u8 = 3;
pub const LONG_ID: u8 = 4;
pub const FLOAT_ID: u8 = 5;
pub const DOUBLE_ID: u8 = 6;
pub const BYTE_ARRAY_ID: u8 = 7;
pub const STRING_ID: u8 = 8;
pub const LIST_ID: u8 = 9;
pub const COMPOUND_ID: u8 = 10;
pub const INT_ARRAY_ID: u8 = 11;
pub const LONG_ARRAY_ID: u8 = 12;

pub const MAX_DEPTH: usize = 512;

#[inline(always)]
pub fn read_u32(data: &mut Cursor<&[u8]>) -> Result<u32, ReadError> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 4 {
        return Err(ReadError::UnexpectedEof);
    }

    data.set_position(data.position() + 4);

    Ok(u32::from_be_bytes([
        remaining_slice[0],
        remaining_slice[1],
        remaining_slice[2],
        remaining_slice[3],
    ]))
}
#[inline(always)]
pub fn read_u16(data: &mut Cursor<&[u8]>) -> Result<u16, ReadError> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 2 {
        return Err(ReadError::UnexpectedEof);
    }

    data.set_position(data.position() + 2);

    Ok(u16::from_be_bytes([remaining_slice[0], remaining_slice[1]]))
}

#[inline(always)]
pub fn read_with_u16_length<'a>(
    data: &mut Cursor<&'a [u8]>,
    width: usize,
) -> Result<&'a [u8], ReadError> {
    let length = read_u16(data)?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(ReadError::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

#[inline(never)]
pub fn read_with_u32_length<'a>(
    data: &mut Cursor<&'a [u8]>,
    width: usize,
) -> Result<&'a [u8], ReadError> {
    let length = read_u32(data)?;
    let length_in_bytes = length as usize * width;
    // make sure we don't read more than the length
    if data.get_ref().len() < data.position() as usize + length_in_bytes {
        return Err(ReadError::UnexpectedEof);
    }
    let start_position = data.position() as usize;
    data.set_position(data.position() + length_in_bytes as u64);
    Ok(&data.get_ref()[start_position..start_position + length_in_bytes])
}

pub fn read_string<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a Mutf8Str, ReadError> {
    let data = read_with_u16_length(data, 1)?;
    Ok(Mutf8Str::from_slice(data))
}

pub fn read_u8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], ReadError> {
    read_with_u32_length(data, 1)
}
pub fn read_i8_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<&'a [i8], ReadError> {
    Ok(slice_u8_into_i8(read_u8_array(data)?))
}

pub fn read_int_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<RawList<'a, i32>, ReadError> {
    let array_bytes = read_with_u32_length(data, 4)?;
    Ok(RawList::new(array_bytes))
}

pub fn read_long_array<'a>(data: &mut Cursor<&'a [u8]>) -> Result<RawList<'a, i64>, ReadError> {
    let array_bytes = read_with_u32_length(data, 8)?;
    Ok(RawList::new(array_bytes))
}

fn slice_u8_into_i8(s: &[u8]) -> &[i8] {
    unsafe { slice::from_raw_parts(s.as_ptr() as *const i8, s.len()) }
}

fn slice_i8_into_u8(s: &[i8]) -> &[u8] {
    unsafe { slice::from_raw_parts(s.as_ptr() as *const u8, s.len()) }
}

pub fn write_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_be_bytes());
}
pub fn write_i32(data: &mut Vec<u8>, value: i32) {
    data.extend_from_slice(&value.to_be_bytes());
}
pub fn write_u16(data: &mut Vec<u8>, value: u16) {
    data.extend_from_slice(&value.to_be_bytes());
}
pub fn write_i8_array(data: &mut Vec<u8>, value: &[i8]) {
    data.extend_from_slice(slice_i8_into_u8(value));
}
pub fn write_string(data: &mut Vec<u8>, value: &Mutf8Str) {
    write_u16(data, value.len() as u16);
    data.extend_from_slice(value.as_bytes());
}

/// Convert a slice of any type into a slice of u8. This will probably return the data as little
/// endian! Use [`slice_into_u8_big_endian`] to get big endian (the endianness that's used in NBT).
pub fn slice_into_u8_native_endian<T>(s: &[T]) -> &[u8] {
    unsafe { slice::from_raw_parts(s.as_ptr() as *const u8, s.len() * std::mem::size_of::<T>()) }
}

/// Convert a slice of any type into a Vec<u8>. This will return the data as big endian (the
/// endianness that's used in NBT).
pub fn slice_into_u8_big_endian<T: SwappableNumber>(s: &[T]) -> Vec<u8> {
    swap_endianness_as_u8::<T>(slice_into_u8_native_endian(s))
}
