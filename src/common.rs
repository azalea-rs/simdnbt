use std::io::Cursor;

use crate::Error;

#[inline(always)]
pub fn read_u32(data: &mut Cursor<&[u8]>) -> Result<u32, Error> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 4 {
        return Err(Error::UnexpectedEof);
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
pub fn read_u16(data: &mut Cursor<&[u8]>) -> Result<u16, Error> {
    let remaining_slice = &data.get_ref()[data.position() as usize..data.get_ref().len()];
    if remaining_slice.len() < 2 {
        return Err(Error::UnexpectedEof);
    }

    data.set_position(data.position() + 2);

    Ok(u16::from_be_bytes([remaining_slice[0], remaining_slice[1]]))
}

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
