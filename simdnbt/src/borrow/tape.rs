use std::{
    alloc::{self, Allocator, Layout},
    fmt::{self, Debug},
    mem,
    ptr::NonNull,
};

use crate::common::{
    BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LONG_ARRAY_ID,
    LONG_ID, SHORT_ID, STRING_ID,
};

const DEFAULT_CAPACITY: usize = 1024;

// this is faster than a Vec mainly because we store a `cur` pointer which
// allows us to avoid having to add the start+length when pushing
#[derive(Debug)]
pub struct MainTape<A: Allocator = alloc::Global> {
    /// The next address we'll be writing to.
    cur: NonNull<TapeElement>,
    /// The last (probably uninitialized) element in the tape.
    end: NonNull<TapeElement>,
    /// The start of the tape.
    ptr: NonNull<u8>,

    alloc: A,
}
impl<A: Allocator> MainTape<A> {
    #[inline]
    pub fn push(&mut self, element: TapeElement) {
        if self.cur == self.end {
            let old_cap = self.capacity();
            let extending_by = old_cap;
            let new_cap = old_cap + extending_by;

            let element_size = mem::size_of::<TapeElement>();
            let old_cap_bytes = old_cap * element_size;
            let new_cap_bytes = new_cap * element_size;
            let new_ptr = unsafe {
                self.alloc.grow(
                    self.ptr,
                    Layout::from_size_align_unchecked(old_cap_bytes, element_size),
                    Layout::from_size_align_unchecked(new_cap_bytes, element_size),
                )
            };
            let new_ptr = new_ptr.expect("allocation failed");

            self.ptr = new_ptr.cast();
            // update cur in case the ptr changed
            self.cur = NonNull::new(self.ptr.as_ptr() as *mut TapeElement).unwrap();
            self.cur = unsafe { self.cur.add(old_cap) };
            // and end has to be updated anyways since we're updating the capacity
            self.end = unsafe { self.cur.add(extending_by) };
        }

        unsafe { self.push_unchecked(element) };
    }

    #[inline]
    pub unsafe fn push_unchecked(&mut self, element: TapeElement) {
        unsafe { self.cur.write(element) };
        self.cur = unsafe { self.cur.add(1) };
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { self.cur.offset_from(self.ptr.cast()) as usize }
    }
    #[inline]
    fn capacity(&self) -> usize {
        unsafe { self.end.offset_from(self.ptr.cast()) as usize }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut TapeElement {
        debug_assert!(index < self.len());
        self.ptr.cast::<TapeElement>().add(index).as_mut()
    }

    #[inline]
    pub fn as_ptr(&self) -> *const TapeElement {
        self.ptr.cast().as_ptr()
    }
}
impl Default for MainTape {
    fn default() -> Self {
        let element_size = mem::size_of::<TapeElement>();
        let ptr = unsafe {
            // this is faster than Layout::array
            alloc::alloc(Layout::from_size_align_unchecked(
                DEFAULT_CAPACITY * element_size,
                element_size,
            ))
        };
        let ptr = NonNull::new(ptr as *mut TapeElement).expect("allocation failed");
        let end = unsafe { ptr.add(DEFAULT_CAPACITY) };
        Self {
            cur: ptr,
            end,
            ptr: ptr.cast(),
            alloc: alloc::Global,
        }
    }
}
impl<A: Allocator> Drop for MainTape<A> {
    fn drop(&mut self) {
        unsafe {
            self.alloc.deallocate(
                self.ptr.cast(),
                Layout::array::<TapeElement>(self.capacity()).unwrap(),
            )
        };
    }
}

#[derive(Clone, Copy)]
pub struct TapeElement(u64);

impl TapeElement {
    pub fn kind(self) -> TapeTagKind {
        let kind_id = (self.0 >> 56) as u8;
        unsafe { mem::transmute::<u8, TapeTagKind>(kind_id) }
    }
    pub fn u8(self) -> u8 {
        self.0 as u8
    }
    pub fn u16(self) -> u16 {
        self.0 as u16
    }
    pub fn u32(self) -> u32 {
        self.0 as u32
    }
    pub fn u64(self) -> u64 {
        self.0
    }

    pub fn approx_len_and_offset(self) -> (u32, u32) {
        ((self.0 >> 32) as u32 & 0xff_ffff, self.0 as u32)
    }
    pub fn ptr<T>(self) -> *const T {
        (self.0 & 0xff_ffff_ffff_ffff) as *const T
    }

    pub fn new_with_approx_len_and_offset(kind: TapeTagKind, approx_len: u32, offset: u32) -> Self {
        let approx_len = approx_len.min(0xff_ffff);
        debug_assert!(approx_len < 2u32.pow(24));
        Self(((kind as u64) << 56) | ((approx_len as u64) << 32) | (offset as u64))
    }
    pub fn set_offset(&mut self, offset: u32) {
        *self = Self::new_with_approx_len_and_offset(
            self.kind(),
            self.approx_len_and_offset().0,
            offset,
        )
    }

    pub fn new_with_u8(kind: TapeTagKind, u8: u8) -> Self {
        Self(((kind as u64) << 56) | u8 as u64)
    }
    pub fn new_with_u16(kind: TapeTagKind, u16: u16) -> Self {
        Self(((kind as u64) << 56) | u16 as u64)
    }
    pub fn new_with_u32(kind: TapeTagKind, u32: u32) -> Self {
        Self(((kind as u64) << 56) | u32 as u64)
    }
    pub fn new_with_ptr<T>(kind: TapeTagKind, ptr: *const T) -> Self {
        Self(((kind as u64) << 56) | ptr as u64)
    }
    /// Create a new TapeElement with the given kind and everything else set to
    /// 0.
    pub fn new_with_0(kind: TapeTagKind) -> Self {
        Self((kind as u64) << 56)
    }
    pub fn new(u64: u64) -> Self {
        Self(u64)
    }
}

impl TapeElement {
    /// Returns how much we should increment the tape index to get to the next
    /// tag.
    ///
    /// # Safety
    /// The element must be a tag and not something else like a continuation of
    /// a long or double.
    pub unsafe fn skip_offset(&self) -> usize {
        match self.kind() {
            TapeTagKind::Compound | TapeTagKind::ListList | TapeTagKind::CompoundList => {
                self.approx_len_and_offset().1 as usize
            }
            _ => 1,
        }
    }
}
impl Debug for TapeElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // just writes the u64
        write!(f, "TapeElement({:#016x})", self.0)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct u56 {
    a: u8,
    b: u16,
    c: u32,
}
impl From<u64> for u56 {
    #[inline]
    fn from(value: u64) -> Self {
        let a = (value >> 48) as u8;
        let b = (value >> 32) as u16;
        let c = value as u32;
        Self { a, b, c }
    }
}
impl From<u56> for u64 {
    #[inline]
    fn from(value: u56) -> Self {
        let a = value.a as u64;
        let b = value.b as u64;
        let c = value.c as u64;
        (a << 48) | (b << 32) | c
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct u24 {
    a: u8,
    b: u16,
}
impl From<u32> for u24 {
    #[inline]
    fn from(value: u32) -> Self {
        let a = (value >> 16) as u8;
        let b = value as u16;
        Self { a, b }
    }
}
impl From<u24> for u32 {
    #[inline]
    fn from(value: u24) -> Self {
        let a = value.a as u32;
        let b = value.b as u32;
        (a << 16) | b
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct UnalignedU64(pub u64);
impl From<u64> for UnalignedU64 {
    #[inline]
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl From<UnalignedU64> for u64 {
    #[inline]
    fn from(value: UnalignedU64) -> Self {
        value.0
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct UnalignedU32(pub u32);
impl From<u32> for UnalignedU32 {
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<UnalignedU32> for u32 {
    #[inline]
    fn from(value: UnalignedU32) -> Self {
        value.0
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct UnalignedU16(pub u16);
impl From<u16> for UnalignedU16 {
    #[inline]
    fn from(value: u16) -> Self {
        Self(value)
    }
}
impl From<UnalignedU16> for u16 {
    #[inline]
    fn from(value: UnalignedU16) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TapeTagKind {
    // putting this at the start makes it slightly faster
    EmptyList = 0,

    Byte = BYTE_ID,
    Short = SHORT_ID,
    Int = INT_ID,
    Long = LONG_ID,
    Float = FLOAT_ID,
    Double = DOUBLE_ID,
    ByteArray = BYTE_ARRAY_ID,
    String = STRING_ID,
    Compound = COMPOUND_ID,
    IntArray = INT_ARRAY_ID,
    LongArray = LONG_ARRAY_ID,

    ByteList,
    ShortList,
    IntList,
    LongList,
    FloatList,
    DoubleList,
    ByteArrayList,
    StringList,
    ListList,
    CompoundList,
    IntArrayList,
    LongArrayList,
}

impl TapeTagKind {
    pub fn is_list(&self) -> bool {
        matches!(
            self,
            TapeTagKind::EmptyList
                | TapeTagKind::ByteList
                | TapeTagKind::ShortList
                | TapeTagKind::IntList
                | TapeTagKind::LongList
                | TapeTagKind::FloatList
                | TapeTagKind::DoubleList
                | TapeTagKind::ByteArrayList
                | TapeTagKind::StringList
                | TapeTagKind::ListList
                | TapeTagKind::CompoundList
                | TapeTagKind::IntArrayList
                | TapeTagKind::LongArrayList
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u56() {
        // top 8 bits are cut off
        let value = 0x1234_5678_9abc_def0;
        let u56 { a, b, c } = u56::from(value);
        assert_eq!(a, 0x34);
        assert_eq!(b, 0x5678);
        assert_eq!(c, 0x9abc_def0);

        let value: u64 = u56 { a, b, c }.into();
        assert_eq!(value, 0x34_5678_9abc_def0);
    }
}
