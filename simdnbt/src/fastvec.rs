use std::{
    alloc::{self, Allocator, Layout},
    mem::{self},
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

impl<T> FastVec<T, alloc::Global> {
    /// Create a new FastVec with the given capacity.
    ///
    /// Note that a capacity of 0 is not supported (as it would require an extra
    /// instruction that's usually unnecessary), and will be treated as 1.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.min(1);

        let element_size = mem::size_of::<T>();
        let ptr = unsafe {
            // this is faster than Layout::array
            alloc::alloc(Layout::from_size_align_unchecked(
                capacity * element_size,
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

impl<T, A: Allocator> FastVec<T, A> {
    #[inline(always)]
    pub fn push(&mut self, element: T) {
        if self.cur == self.end {
            self.grow()
        }

        unsafe { self.push_unchecked(element) };
    }

    #[inline]
    pub unsafe fn push_unchecked(&mut self, element: T) {
        unsafe { self.cur.write(element) };
        self.cur = unsafe { self.cur.add(1) };
    }

    #[inline]
    fn grow(&mut self) {
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
        self.ptr.cast::<T>().add(index).as_mut()
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.cast().as_ptr()
    }

    pub fn to_vec(self) -> Vec<T> {
        let vec = unsafe {
            Vec::from_raw_parts(
                self.ptr.cast().as_ptr() as *mut T,
                self.len(),
                self.capacity(),
            )
        };
        // the allocation was moved so don't drop it
        mem::forget(self);
        vec
    }
}

const DEFAULT_CAPACITY: usize = 1024;

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
        fastvec.to_vec()
    }
}
