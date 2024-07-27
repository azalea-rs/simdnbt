use std::{
    fmt::{self, Debug},
    mem,
};

use crate::common::{
    BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, FLOAT_ID, INT_ARRAY_ID, INT_ID, LONG_ARRAY_ID,
    LONG_ID, SHORT_ID, STRING_ID,
};

#[derive(Debug)]
pub struct MainTape {
    elements: Vec<TapeElement>,
}
impl MainTape {
    #[inline]
    pub fn push(&mut self, element: TapeElement) {
        self.elements.push(element);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut TapeElement {
        self.elements.get_unchecked_mut(index)
    }

    #[inline]
    pub fn as_ptr(&self) -> *const TapeElement {
        self.elements.as_ptr()
    }
}
impl Default for MainTape {
    fn default() -> Self {
        Self {
            elements: Vec::with_capacity(1024),
        }
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
        debug_assert!(approx_len < 2u32.pow(24));
        Self((kind as u64) << 56 | (approx_len as u64) << 32 | (offset as u64))
    }
    pub fn set_offset(&mut self, offset: u32) {
        *self = Self::new_with_approx_len_and_offset(
            self.kind(),
            self.approx_len_and_offset().0,
            offset,
        )
    }

    pub fn new_with_u8(kind: TapeTagKind, u8: u8) -> Self {
        Self((kind as u64) << 56 | u8 as u64)
    }
    pub fn new_with_u16(kind: TapeTagKind, u16: u16) -> Self {
        Self((kind as u64) << 56 | u16 as u64)
    }
    pub fn new_with_u32(kind: TapeTagKind, u32: u32) -> Self {
        Self((kind as u64) << 56 | u32 as u64)
    }
    pub fn new_with_ptr<T>(kind: TapeTagKind, ptr: *const T) -> Self {
        Self((kind as u64) << 56 | ptr as u64)
    }
    /// Create a new TapeElement with the given kind and everything else set to 0.
    pub fn new_with_0(kind: TapeTagKind) -> Self {
        Self((kind as u64) << 56)
    }
    pub fn new(u64: u64) -> Self {
        Self(u64)
    }
}

impl TapeElement {
    /// Returns how much we should increment the tape index to get to the next tag.
    ///
    /// # Safety
    /// The element must be a tag and not something else like a continuation of a long or double.
    pub unsafe fn skip_offset(&self) -> usize {
        match self.kind() {
            TapeTagKind::Long | TapeTagKind::Double => 2,
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
#[repr(packed)]
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
#[repr(packed)]
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
#[repr(packed)]
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
#[repr(packed)]
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

    EmptyList,
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
