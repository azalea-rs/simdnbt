use std::{
    fmt::{self, Debug, Pointer},
    marker::PhantomData,
    ops::Deref,
    ptr::Pointee,
};

#[repr(packed(2))]
pub struct Slice16Bit<'a, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    ptr: *const u8,
    len: u16,
    _marker: PhantomData<&'a T>,
}

#[repr(packed(4))]
pub struct Slice32Bit<'a, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    ptr: *const u8,
    len: u32,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Slice32Bit<'a, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    pub fn new(ptr: *const u8, len: u32) -> Self {
        Self {
            ptr,
            len,
            _marker: PhantomData,
        }
    }
    pub fn len(&self) -> u32 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
}
impl<T> Deref for Slice32Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // self.ptr.with_metadata_of(&self.len).as_ptr()
        let p = std::ptr::from_raw_parts(self.ptr as *const (), (self.len as usize).into());
        unsafe { &*(p as *const T) }
    }
}
impl<T> Debug for Slice32Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (&**self).fmt(f)
    }
}
impl<T> PartialEq for Slice32Bit<'_, T>
where
    T: PartialEq,
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn eq(&self, other: &Self) -> bool {
        (&**self).eq(&**other)
    }
}
impl<T> Clone for Slice32Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            len: self.len,
            _marker: PhantomData,
        }
    }
}
impl<T> Copy for Slice32Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
}
impl<T> Default for Slice32Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn default() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
            _marker: PhantomData,
        }
    }
}
impl<'a, T> From<&'a [T]> for Slice32Bit<'a, [T]> {
    fn from(slice: &'a [T]) -> Self {
        Self {
            ptr: slice.as_ptr() as *const u8,
            len: slice.len() as u32,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Slice16Bit<'a, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    pub fn new(ptr: *const u8, len: u16) -> Self {
        Self {
            ptr,
            len,
            _marker: PhantomData,
        }
    }
    pub fn len(&self) -> u16 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
}
impl<T> Deref for Slice16Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // self.ptr.with_metadata_of(&self.len).as_ptr()
        let p = std::ptr::from_raw_parts(self.ptr as *const (), (self.len as usize).into());
        unsafe { &*(p as *const T) }
    }
}
impl<T> Debug for Slice16Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (&**self).fmt(f)
    }
}
impl<T> PartialEq for Slice16Bit<'_, T>
where
    T: PartialEq,
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn eq(&self, other: &Self) -> bool {
        (&**self).eq(&**other)
    }
}
impl<T> Clone for Slice16Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            len: self.len,
            _marker: PhantomData,
        }
    }
}
impl<T> Copy for Slice16Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
}
impl<T> Default for Slice16Bit<'_, T>
where
    T: ?Sized + Pointee,
    <T as Pointee>::Metadata: From<usize>,
{
    fn default() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
            _marker: PhantomData,
        }
    }
}
impl<'a, T> From<&'a [T]> for Slice16Bit<'a, [T]> {
    fn from(slice: &'a [T]) -> Self {
        Self {
            ptr: slice.as_ptr() as *const u8,
            len: slice.len() as u16,
            _marker: PhantomData,
        }
    }
}
