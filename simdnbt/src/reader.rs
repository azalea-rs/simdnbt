#![doc(hidden)]

use std::{
    io::Cursor,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use crate::error::UnexpectedEofError;

/// An internal type for efficiently reading bytes, and is sometimes faster than
/// `Cursor<&[u8]>`.
pub struct Reader<'a> {
    pub cur: *const u8,
    /// pointer to after the last byte (so remaining=end-cur)
    end: *const u8,

    _marker: PhantomData<&'a ()>,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Self {
            cur: data.as_ptr(),
            end: unsafe { data.as_ptr().add(data.len()) },
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn end_addr(&self) -> usize {
        self.end as usize
    }

    pub fn ensure_can_read(&self, size: usize) -> Result<(), UnexpectedEofError> {
        let data_addr = self.cur as usize;
        let end_addr = self.end as usize;
        if data_addr + size > end_addr {
            Err(UnexpectedEofError)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub unsafe fn read_type_unchecked<T>(&mut self) -> T {
        let value = unsafe { self.cur.cast::<T>().read_unaligned() };
        self.cur = unsafe { self.cur.add(mem::size_of::<T>()) };
        value
    }

    #[inline]
    pub fn read_type<T: Copy>(&mut self) -> Result<T, UnexpectedEofError> {
        let addr = self.cur;

        // it's faster to add and then check for eof. it does leave our pointer in a bad
        // state if it fails, but we aren't going to dereference it in that case
        // anyways.
        self.cur = unsafe { self.cur.add(mem::size_of::<T>()) };
        if self.cur > self.end {
            return Err(UnexpectedEofError);
        }

        let value = unsafe { addr.cast::<T>().read_unaligned() };
        Ok(value)
    }

    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, UnexpectedEofError> {
        self.read_type()
    }
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8, UnexpectedEofError> {
        self.read_u8().map(|x| x as i8)
    }

    #[inline]
    pub fn peek_u8(&self) -> Result<u8, UnexpectedEofError> {
        let addr = self.cur;
        if addr >= self.end {
            return Err(UnexpectedEofError);
        }
        Ok(unsafe { addr.read_unaligned() })
    }

    #[inline]
    pub fn read_u16(&mut self) -> Result<u16, UnexpectedEofError> {
        self.read_type::<u16>().map(u16::to_be)
    }
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16, UnexpectedEofError> {
        self.read_u16().map(|x| x as i16)
    }

    #[inline]
    pub fn read_u32(&mut self) -> Result<u32, UnexpectedEofError> {
        self.read_type::<u32>().map(u32::to_be)
    }
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32, UnexpectedEofError> {
        self.read_u32().map(|x| x as i32)
    }

    #[inline]
    pub fn read_u64(&mut self) -> Result<u64, UnexpectedEofError> {
        self.read_type::<u64>().map(u64::to_be)
    }
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64, UnexpectedEofError> {
        self.read_u64().map(|x| x as i64)
    }

    #[inline]
    pub fn read_f32(&mut self) -> Result<f32, UnexpectedEofError> {
        self.read_u32().map(f32::from_bits)
    }

    #[inline]
    pub fn read_f64(&mut self) -> Result<f64, UnexpectedEofError> {
        self.read_u64().map(f64::from_bits)
    }

    #[inline]
    pub fn skip(&mut self, size: usize) -> Result<(), UnexpectedEofError> {
        self.cur = unsafe { self.cur.add(size) };
        if self.cur > self.end {
            return Err(UnexpectedEofError);
        }

        Ok(())
    }

    #[inline]
    pub unsafe fn skip_unchecked(&mut self, size: usize) {
        self.cur = unsafe { self.cur.add(size) };
    }

    #[inline]
    pub fn read_slice(&mut self, size: usize) -> Result<&'a [u8], UnexpectedEofError> {
        self.ensure_can_read(size)?;
        let slice = unsafe { std::slice::from_raw_parts(self.cur, size) };
        unsafe { self.skip_unchecked(size) };
        Ok(slice)
    }
}

impl<'a> From<&'a [u8]> for Reader<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self::new(data)
    }
}

pub struct ReaderFromCursor<'a: 'cursor, 'cursor> {
    reader: Reader<'a>,
    original_cursor: &'cursor mut Cursor<&'a [u8]>,
}

impl<'a, 'cursor> ReaderFromCursor<'a, 'cursor> {
    pub fn new(cursor: &'cursor mut Cursor<&'a [u8]>) -> Self {
        let data = cursor.get_ref();
        Self {
            reader: Reader::new(data[cursor.position() as usize..].as_ref()),
            original_cursor: cursor,
        }
    }
}
impl Drop for ReaderFromCursor<'_, '_> {
    fn drop(&mut self) {
        self.original_cursor.set_position(
            (self.reader.cur as usize - self.original_cursor.get_ref().as_ptr() as usize) as u64,
        );
    }
}

impl<'a> Deref for ReaderFromCursor<'a, '_> {
    type Target = Reader<'a>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl DerefMut for ReaderFromCursor<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}
