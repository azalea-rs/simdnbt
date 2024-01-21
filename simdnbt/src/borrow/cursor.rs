use crate::{
    common::{read_u8_array, slice_u8_into_i8},
    raw_list::RawList,
    Error, Mutf8Str,
};

pub struct McCursor<'a> {
    slice: &'a [u8],
}

impl<'a> McCursor<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    #[inline(always)]
    fn try_read_exact(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if n > self.slice.len() {
            return Err(Error::UnexpectedEof);
        }
        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
        // fulfills the requirements of `split_at_unchecked`.
        let (a, b) = unsafe { self.slice.split_at_unchecked(n) };
        self.slice = b;
        Ok(a)
    }

    #[inline]
    pub fn advance(&mut self, n: usize) -> Result<(), Error> {
        if n > self.slice.len() {
            return Err(Error::UnexpectedEof);
        }
        self.slice = &self.slice[n..];
        Ok(())
    }

    #[inline]
    pub fn read_i8(&mut self) -> Result<i8, Error> {
        Ok(<i8>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<i8>())?.as_ptr()
                as *const [u8; core::mem::size_of::<i8>()])
        }))
    }

    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, Error> {
        self.read_i8().map(|x| x as u8)
    }

    #[inline]
    pub fn read_i16(&mut self) -> Result<i16, Error> {
        Ok(<i16>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<i16>())?.as_ptr()
                as *const [u8; core::mem::size_of::<i16>()])
        }))
    }

    #[inline]
    pub fn read_i32(&mut self) -> Result<i32, Error> {
        Ok(<i32>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<i32>())?.as_ptr()
                as *const [u8; core::mem::size_of::<i32>()])
        }))
    }

    #[inline]
    pub fn read_u32(&mut self) -> Result<u32, Error> {
        self.read_i32().map(|x| x as u32)
    }

    #[inline]
    pub fn read_i64(&mut self) -> Result<i64, Error> {
        Ok(<i64>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<i64>())?.as_ptr()
                as *const [u8; core::mem::size_of::<i64>()])
        }))
    }

    #[inline]
    pub fn read_f32(&mut self) -> Result<f32, Error> {
        Ok(<f32>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<f32>())?.as_ptr()
                as *const [u8; core::mem::size_of::<f32>()])
        }))
    }

    #[inline]
    pub fn read_f64(&mut self) -> Result<f64, Error> {
        Ok(<f64>::from_be_bytes(unsafe {
            *(self.try_read_exact(core::mem::size_of::<f64>())?.as_ptr()
                as *const [u8; core::mem::size_of::<f64>()])
        }))
    }

    pub fn read_with_u32_length<'b>(&mut self, width: usize) -> Result<&'a [u8], Error> {
        let length = self.read_i32()?;
        let length_in_bytes = length as usize * width;
        // make sure we don't read more than the length
        if self.slice.len() < length_in_bytes {
            return Err(Error::UnexpectedEof);
        }
        let (a, b) = self.slice.split_at(length_in_bytes);
        self.slice = b;
        Ok(a)
    }

    pub fn read_with_u16_length<'b>(&mut self, width: usize) -> Result<&'a [u8], Error> {
        let length = self.read_i16()? as u16;
        let length_in_bytes = length as usize * width;
        // make sure we don't read more than the length
        if self.slice.len() < length_in_bytes {
            return Err(Error::UnexpectedEof);
        }
        self.try_read_exact(length_in_bytes)
    }

    pub fn read_string(&mut self) -> Result<&'a Mutf8Str, Error> {
        let data = self.read_with_u16_length(1)?;
        Ok(Mutf8Str::from_slice(data))
    }

    pub fn read_int_array(&mut self) -> Result<RawList<'a, i32>, Error> {
        let array_bytes = self.read_with_u32_length(4)?;
        Ok(RawList::new(array_bytes))
    }

    pub fn read_long_array(&mut self) -> Result<RawList<'a, i64>, Error> {
        let array_bytes = self.read_with_u32_length(8)?;
        Ok(RawList::new(array_bytes))
    }

    pub fn read_u8_array(&mut self) -> Result<&'a [u8], Error> {
        self.read_with_u32_length(1)
    }
    pub fn read_i8_array(&mut self) -> Result<&'a [i8], Error> {
        Ok(slice_u8_into_i8(self.read_u8_array()?))
    }
}
