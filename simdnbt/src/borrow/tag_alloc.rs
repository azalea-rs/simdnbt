//! Some tags, like compounds and arrays, contain other tags. The naive approach would be to just
//! use `Vec`s or `HashMap`s, but this is inefficient and leads to many small allocations.
//!
//! Instead, the idea for this is essentially that we'd have two big Vec for every tag (one for
//! named tags and one for unnamed tags), and then compounds/arrays simply contain a slice of this
//! vec.
//!
//! This almost works. but there's two main issues:
//! - compounds aren't length-prefixed, so we can't pre-allocate at the beginning of compounds for
//! the rest of that compound
//! - resizing a vec might move it in memory, invalidating all of our slices to it
//!
//! solving the first problem isn't that hard, since we can have a separate vec for every "depth"
//! (so compounds in compounds don't share the same vec).
//! to solve the second problem, i chose to implement a special data structure
//! that relies on low-level allocations so we can guarantee that our allocations don't move in memory.

use std::{
    alloc::{self, Layout},
    cell::UnsafeCell,
    fmt,
    mem::MaybeUninit,
    ptr::NonNull,
};

use crate::{raw_list::RawList, Mutf8Str};

use super::{NbtCompound, NbtList, NbtTag};

// this value appears to have the best results on my pc when testing with complex_player.dat
const MIN_ALLOC_SIZE: usize = 1024;

/// The data structure that contains all the parsed NBT tags. This must stay in scope for as long
/// as the borrowed NBT exists.
#[derive(Default)]
pub struct TagAllocator<'a>(UnsafeCell<TagAllocatorImpl<'a>>);
impl<'a> TagAllocator<'a> {
    pub fn new() -> Self {
        Self(UnsafeCell::new(TagAllocatorImpl::new()))
    }
    // shhhhh
    #[allow(clippy::mut_from_ref)]
    pub fn get(&self) -> &mut TagAllocatorImpl<'a> {
        unsafe { self.0.get().as_mut().unwrap_unchecked() }
    }
}

#[derive(Default)]
pub struct TagAllocatorImpl<'a> {
    pub named: IndividualTagAllocatorWithDepth<(&'a Mutf8Str, NbtTag<'a>)>,

    // lists of lists
    pub unnamed_list: IndividualTagAllocatorWithDepth<NbtList<'a>>,
    // lists of compounds
    pub unnamed_compound: IndividualTagAllocatorWithDepth<NbtCompound<'a>>,

    pub unnamed_bytearray: IndividualTagAllocator<&'a [u8]>,
    pub unnamed_string: IndividualTagAllocator<&'a Mutf8Str>,
    pub unnamed_intarray: IndividualTagAllocator<RawList<'a, i32>>,
    pub unnamed_longarray: IndividualTagAllocator<RawList<'a, i64>>,
}

impl<'a> TagAllocatorImpl<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct IndividualTagAllocator<T> {
    current: TagsAllocation<T>,
    // we keep track of old allocations so we can deallocate them later
    previous: Vec<TagsAllocation<T>>,
}
impl<T> IndividualTagAllocator<T>
where
    T: Clone,
{
    #[inline]
    pub fn start(&mut self) -> ContiguousTagsAllocator<T> {
        let alloc = self.current.clone();

        start_allocating_tags(alloc)
    }
    #[inline]
    pub fn finish<'a>(&mut self, alloc: ContiguousTagsAllocator<T>) -> &'a [T] {
        finish_allocating_tags(alloc, &mut self.current, &mut self.previous)
    }
}
impl<T> Default for IndividualTagAllocator<T> {
    fn default() -> Self {
        Self {
            current: Default::default(),
            previous: Default::default(),
        }
    }
}
impl<T> Drop for IndividualTagAllocator<T> {
    fn drop(&mut self) {
        self.current.deallocate();
        self.previous
            .iter_mut()
            .for_each(TagsAllocation::deallocate);
    }
}

pub struct IndividualTagAllocatorWithDepth<T> {
    // it's a vec because of the depth thing mentioned earlier, index in the vec = depth
    current: Vec<TagsAllocation<T>>,
    // we also have to keep track of old allocations so we can deallocate them later
    previous: Vec<Vec<TagsAllocation<T>>>,
}
impl<T> IndividualTagAllocatorWithDepth<T>
where
    T: Clone,
{
    #[inline]
    pub fn start(&mut self, depth: usize) -> ContiguousTagsAllocator<T> {
        // make sure we have enough space for this depth
        // (also note that depth is reused for compounds and arrays so we might have to push
        // more than once)
        for _ in self.current.len()..=depth {
            self.current.push(Default::default());
            self.previous.push(Default::default());
        }

        let alloc = self.current[depth].clone();

        start_allocating_tags(alloc)
    }
    #[inline]
    pub fn finish<'a>(&mut self, alloc: ContiguousTagsAllocator<T>, depth: usize) -> &'a [T] {
        finish_allocating_tags(alloc, &mut self.current[depth], &mut self.previous[depth])
    }
}
impl<T> Default for IndividualTagAllocatorWithDepth<T> {
    fn default() -> Self {
        Self {
            current: Default::default(),
            previous: Default::default(),
        }
    }
}
impl<T> Drop for IndividualTagAllocatorWithDepth<T> {
    fn drop(&mut self) {
        self.current.iter_mut().for_each(TagsAllocation::deallocate);
        self.previous
            .iter_mut()
            .flatten()
            .for_each(TagsAllocation::deallocate);
    }
}

#[inline]
fn start_allocating_tags<T>(alloc: TagsAllocation<T>) -> ContiguousTagsAllocator<T> {
    let is_new_allocation = alloc.cap == 0;
    ContiguousTagsAllocator {
        alloc,
        is_new_allocation,
        size: 0,
    }
}
#[inline]
fn finish_allocating_tags<'a, T>(
    alloc: ContiguousTagsAllocator<T>,
    current_alloc: &mut TagsAllocation<T>,
    previous_allocs: &mut Vec<TagsAllocation<T>>,
) -> &'a [T] {
    let slice = unsafe {
        std::slice::from_raw_parts(
            alloc
                .alloc
                .ptr
                .as_ptr()
                .add(alloc.alloc.len)
                .sub(alloc.size),
            alloc.size,
        )
    };

    let previous_allocation_at_that_depth = std::mem::replace(current_alloc, alloc.alloc);
    if alloc.is_new_allocation {
        previous_allocs.push(previous_allocation_at_that_depth);
    }

    slice
}

#[derive(Clone)]
pub struct TagsAllocation<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}
impl<T> Default for TagsAllocation<T> {
    fn default() -> Self {
        Self {
            ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
        }
    }
}
impl<T> TagsAllocation<T> {
    fn deallocate(&mut self) {
        if self.cap == 0 {
            return;
        }

        // call drop on the tags too
        unsafe {
            std::ptr::drop_in_place(std::slice::from_raw_parts_mut(
                self.ptr.as_ptr().cast::<T>(),
                self.len,
            ));
        }

        unsafe {
            alloc::dealloc(
                self.ptr.as_ptr().cast(),
                Layout::array::<T>(self.cap).unwrap(),
            )
        }
    }
}

// this is created when we start allocating a compound tag
pub struct ContiguousTagsAllocator<T> {
    alloc: TagsAllocation<T>,
    /// whether we created a new allocation for this compound (as opposed to reusing an existing
    /// one).
    /// this is used to determine whether we're allowed to deallocate it when growing, and whether
    /// we should add this allocation to `all_allocations`
    is_new_allocation: bool,
    /// the size of this individual compound allocation. the size of the full allocation is in
    /// `alloc.len`.
    size: usize,
}

impl<T> ContiguousTagsAllocator<T> {
    /// Grow the capacity to the new amount.
    ///
    /// # Safety
    /// Must be at least the current capacity.
    unsafe fn grow_to(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.alloc.cap, "{new_cap} < {}", self.alloc.cap);

        let new_layout = Layout::array::<T>(new_cap).unwrap();

        let new_ptr = if self.is_new_allocation && self.alloc.ptr != NonNull::dangling() {
            let old_ptr = self.alloc.ptr.as_ptr();
            let old_cap = self.alloc.cap;
            let old_layout = Layout::array::<T>(old_cap).unwrap();
            unsafe { alloc::realloc(old_ptr as *mut u8, old_layout, new_cap) }
        } else {
            self.is_new_allocation = true;
            unsafe { alloc::alloc(new_layout) }
        } as *mut T;

        // copy the last `size` elements from the old allocation to the new one
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.alloc.ptr.as_ptr().sub(self.size),
                new_ptr,
                self.size,
            )
        };

        self.alloc.ptr = NonNull::new(new_ptr).unwrap();
        self.alloc.cap = new_cap;
        self.alloc.len = self.size;
    }

    #[inline(never)]
    fn grow(&mut self) {
        let new_cap = if self.is_new_allocation {
            // this makes sure we don't allocate 0 bytes
            std::cmp::max(self.alloc.cap * 2, MIN_ALLOC_SIZE)
        } else {
            // reuse the previous cap, since it's not unlikely that we'll have another compound
            // with a similar
            self.alloc.cap
        };

        unsafe { self.grow_to(new_cap) };
    }

    #[allow(dead_code)]
    pub fn allocate(&mut self, size: usize) -> &mut [MaybeUninit<T>] {
        // check if we need to reallocate
        if self.alloc.len + size > self.alloc.cap {
            // unsafe { self.grow_to(self.size + size) }
            let new_cap = std::cmp::max(self.alloc.cap, self.size + size);
            unsafe { self.grow_to(new_cap) }
        }

        let start = unsafe { self.alloc.ptr.as_ptr().add(self.alloc.len) };

        self.alloc.len += size;
        self.size += size;

        unsafe { std::slice::from_raw_parts_mut(start.cast(), size) }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        // check if we need to reallocate
        if self.alloc.len == self.alloc.cap {
            self.grow();
        }

        // push the new tag
        unsafe {
            let end = self.alloc.ptr.as_ptr().add(self.alloc.len);
            std::ptr::write(end, value);
        }
        self.alloc.len += 1;
        self.size += 1;
    }
}

impl<'a> fmt::Debug for TagAllocator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TagAllocator").finish()
    }
}
