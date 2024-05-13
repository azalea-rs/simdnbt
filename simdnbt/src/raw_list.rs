use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    mem,
};

use crate::{
    swap_endianness::{swap_endianness, swap_endianness_as_u8, SwappableNumber},
    thin_slices::Slice32Bit,
};

/// A list of numbers that's kept as big-endian in memory.

// #[derive(Debug, PartialEq, Clone)]
#[repr(transparent)]
pub struct RawList<'a, T> {
    data: Slice32Bit<'a, [u8]>,
    _marker: PhantomData<T>,
}
impl<'a, T> RawList<'a, T> {
    pub fn new(data: Slice32Bit<'a, [u8]>) -> Self {
        Self {
            data,
            _marker: PhantomData,
        }
    }
    pub fn new_from_slice(data: &'a [u8]) -> Self {
        Self::new(Slice32Bit::new(data.as_ptr(), data.len() as u32))
    }

    pub fn len(&self) -> usize {
        (self.data.len() / mem::size_of::<T>() as u32) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T: SwappableNumber> RawList<'_, T> {
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Copy + SwappableNumber,
    {
        swap_endianness(self.data)
    }

    pub fn to_little_endian(&self) -> Vec<u8> {
        swap_endianness_as_u8::<T>(self.data)
    }

    pub fn as_big_endian(&self) -> &[u8] {
        &self.data
    }
}

impl<T> IntoIterator for RawList<'_, T>
where
    T: Copy + SwappableNumber,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_vec().into_iter()
    }
}
impl<T> IntoIterator for &RawList<'_, T>
where
    T: Copy + SwappableNumber,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_vec().into_iter()
    }
}

impl<T> Debug for RawList<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}
impl<T> PartialEq for RawList<'_, T>
where
    T: PartialEq + Copy + SwappableNumber,
{
    fn eq(&self, other: &Self) -> bool {
        self.to_vec() == other.to_vec()
    }
}
impl<T> Clone for RawList<'_, T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            _marker: PhantomData,
        }
    }
}
