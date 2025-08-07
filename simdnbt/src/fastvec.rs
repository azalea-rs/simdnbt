use std::{
    alloc::{self, Allocator, Layout},
    fmt::Debug,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// A slightly faster (but more limited) append-only Vec.
#[derive(Debug)]
pub struct FastVec<T, A: Allocator = alloc::Global> {
    /// The next address we'll be writing to.
    cur: NonNull<T>,
    /// The last (probably uninitialized) element.
    end: NonNull<T>,
    /// The start of the vec.
    ptr: NonNull<u8>,

    alloc: A,
}

impl<T, A: Allocator> FastVec<T, A> {
    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        let capacity = capacity.max(1);

        let element_size = mem::size_of::<T>();
        let ptr = unsafe {
            // this is faster than Layout::array
            alloc::alloc(Layout::from_size_align_unchecked(
                capacity * element_size,
                element_size,
            ))
        };
        let ptr = NonNull::new(ptr as *mut T).expect("allocation failed");
        let end = unsafe { ptr.add(capacity) };
        Self {
            cur: ptr,
            end,
            ptr: ptr.cast(),
            alloc,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, element: T) {
        if self.cur == self.end {
            unsafe { self.grow_at_max() }
        }

        unsafe { self.push_unchecked(element) };
    }

    #[inline]
    pub unsafe fn push_unchecked(&mut self, element: T) {
        debug_assert!(self.cur != self.end);

        unsafe { self.cur.write(element) };
        self.cur = unsafe { self.cur.add(1) };
    }

    /// Grow the capacity slightly more efficiently if we already know the
    /// length is the same as the capacity.
    #[inline]
    unsafe fn grow_at_max(&mut self) {
        let old_cap = self.capacity();
        let extending_by = old_cap;
        let new_cap = old_cap + extending_by;

        let element_size = mem::size_of::<T>();
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
        self.cur = NonNull::new(self.ptr.as_ptr() as *mut T).unwrap();
        self.cur = unsafe { self.cur.add(old_cap) };
        // and end has to be updated anyways since we're updating the capacity
        self.end = unsafe { self.cur.add(extending_by) };
    }

    #[inline]
    fn grow(&mut self) {
        let old_cap = self.capacity();
        let len = self.len();
        let extending_by = old_cap;
        let new_cap = old_cap + extending_by;

        let element_size = mem::size_of::<T>();
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
        let base = NonNull::new(self.ptr.as_ptr() as *mut T).unwrap();
        self.cur = unsafe { base.add(len) };
        // and end has to be updated anyways since we're updating the capacity
        self.end = unsafe { base.add(new_cap) };
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
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        debug_assert!(index < self.len());
        unsafe { self.ptr.cast::<T>().add(index).as_mut() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.cast().as_ptr()
    }

    pub fn into_vec(self) -> Vec<T> {
        let vec =
            unsafe { Vec::from_raw_parts(self.ptr.cast().as_ptr(), self.len(), self.capacity()) };
        // the allocation was moved so don't drop it
        mem::forget(self);
        vec
    }

    pub fn extend_from_slice(&mut self, slice: &[T])
    where
        T: Copy,
    {
        self.reserve(slice.len());
        unsafe { self.extend_from_slice_unchecked(slice) };
    }

    /// Extend your FastVec with a slice without checking if it has enough
    /// capacity.
    ///
    /// This optimization is barely measurable, but it does make it slightly
    /// faster!
    ///
    /// # Safety
    ///
    /// You must reserve enough capacity in the Vec before calling this
    /// function.
    pub unsafe fn extend_from_slice_unchecked(&mut self, slice: &[T])
    where
        T: Copy,
    {
        for element in slice {
            // SAFETY: We just reserved enough space for the slice
            unsafe { self.push_unchecked(*element) };
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        let new_len = self.len() + additional;
        while new_len > self.capacity() {
            self.grow()
        }
    }

    pub unsafe fn from_raw_parts_in(ptr: *mut T, len: usize, capacity: usize, alloc: A) -> Self {
        if capacity == 0 {
            // capacity 0 means the vec is probably a null pointer, create it ourselves
            return Self::with_capacity_in(0, alloc);
        }

        let ptr = NonNull::new(ptr).expect("null pointer");
        let cur = unsafe { ptr.add(len) };
        let end = unsafe { ptr.add(capacity) };
        Self {
            cur,
            end,
            ptr: ptr.cast(),
            alloc,
        }
    }
}

impl<T> FastVec<T, alloc::Global> {
    /// Create a new FastVec with the given capacity.
    ///
    /// Note that a capacity of 0 is not supported (as it would require an extra
    /// instruction that's usually unnecessary), and will be treated as 1.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity, alloc::Global)
    }

    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize, capacity: usize) -> Self {
        unsafe { Self::from_raw_parts_in(ptr, len, capacity, alloc::Global) }
    }
}

const DEFAULT_CAPACITY: usize = 1;

impl<T> Default for FastVec<T> {
    fn default() -> Self {
        let element_size = mem::size_of::<T>();
        let ptr = unsafe {
            // this is faster than Layout::array
            alloc::alloc(Layout::from_size_align_unchecked(
                DEFAULT_CAPACITY * element_size,
                element_size,
            ))
        };
        let ptr = NonNull::new(ptr as *mut T).expect("allocation failed");
        let end = unsafe { ptr.add(DEFAULT_CAPACITY) };
        Self {
            cur: ptr,
            end,
            ptr: ptr.cast(),
            alloc: alloc::Global,
        }
    }
}
impl<T, A: Allocator> Drop for FastVec<T, A> {
    fn drop(&mut self) {
        unsafe {
            self.alloc.deallocate(
                self.ptr.cast(),
                Layout::array::<T>(self.capacity()).unwrap(),
            )
        };
    }
}

impl<T> From<Vec<T>> for FastVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut fastvec = FastVec::with_capacity(vec.len());
        for element in vec {
            fastvec.push(element);
        }
        fastvec
    }
}
impl<T> From<FastVec<T>> for Vec<T> {
    fn from(fastvec: FastVec<T>) -> Self {
        fastvec.into_vec()
    }
}

pub struct FastVecFromVec<'orig, T> {
    fastvec: ManuallyDrop<FastVec<T>>,
    original: &'orig mut Vec<T>,
}

impl<'orig, T> FastVecFromVec<'orig, T> {
    pub fn new(data: &'orig mut Vec<T>) -> Self {
        Self {
            fastvec: ManuallyDrop::new(unsafe {
                FastVec::from_raw_parts(data.as_mut_ptr(), data.len(), data.capacity())
            }),
            original: data,
        }
    }
}
impl<T> Drop for FastVecFromVec<'_, T> {
    /// Move the FastVec contents back into the original Vec.
    fn drop(&mut self) {
        // we intentionally don't drop anything here
        unsafe {
            let new_vec = ManuallyDrop::take(&mut self.fastvec).into_vec();
            (self.original as *mut Vec<T>).write(new_vec);
        }
    }
}

impl<T> Deref for FastVecFromVec<'_, T> {
    type Target = FastVec<T>;

    fn deref(&self) -> &Self::Target {
        &self.fastvec
    }
}
impl<T> DerefMut for FastVecFromVec<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fastvec
    }
}
