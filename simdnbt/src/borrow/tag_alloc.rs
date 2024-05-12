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
    fmt,
    ptr::NonNull,
};

use crate::Mutf8Str;

use super::{NbtCompound, NbtList, NbtTag};

// this value appears to have the best results on my pc when testing with complex_player.dat
const MIN_ALLOC_SIZE: usize = 1024;

#[derive(Default)]
pub struct TagAllocator<'a> {
    // it's a vec because of the depth thing mentioned earlier, index in the vec = depth
    named_tags: Vec<TagsAllocation<(&'a Mutf8Str, NbtTag<'a>)>>,
    // we also have to keep track of old allocations so we can deallocate them later
    previous_named_tags: Vec<Vec<TagsAllocation<(&'a Mutf8Str, NbtTag<'a>)>>>,

    // so remember earlier when i said the depth thing is only necessary because compounds aren't
    // length prefixed? ... well soooo i decided to make arrays store per-depth separately too to
    // avoid exploits where an array with a big length is sent to force it to immediately allocate
    // a lot
    unnamed_list_tags: Vec<TagsAllocation<NbtList<'a>>>,
    previous_unnamed_list_tags: Vec<Vec<TagsAllocation<NbtList<'a>>>>,

    unnamed_compound_tags: Vec<TagsAllocation<NbtCompound<'a>>>,
    previous_unnamed_compound_tags: Vec<Vec<TagsAllocation<NbtCompound<'a>>>>,
}

impl<'a> TagAllocator<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_named_tags(
        &mut self,
        depth: usize,
    ) -> ContiguousTagsAllocator<(&'a Mutf8Str, NbtTag<'a>)> {
        start_allocating_tags_with_depth(depth, &mut self.named_tags, &mut self.previous_named_tags)
    }
    pub fn finish_named_tags(
        &mut self,
        alloc: ContiguousTagsAllocator<(&'a Mutf8Str, NbtTag<'a>)>,
        depth: usize,
    ) -> &'a [(&'a Mutf8Str, NbtTag)] {
        finish_allocating_tags_with_depth(
            alloc,
            depth,
            &mut self.named_tags,
            &mut self.previous_named_tags,
        )
    }

    pub fn start_unnamed_list_tags(
        &mut self,
        depth: usize,
    ) -> ContiguousTagsAllocator<NbtList<'a>> {
        start_allocating_tags_with_depth(
            depth,
            &mut self.unnamed_list_tags,
            &mut self.previous_unnamed_list_tags,
        )
    }
    pub fn finish_unnamed_list_tags(
        &mut self,
        alloc: ContiguousTagsAllocator<NbtList<'a>>,
        depth: usize,
    ) -> &'a [NbtList<'a>] {
        finish_allocating_tags_with_depth(
            alloc,
            depth,
            &mut self.unnamed_list_tags,
            &mut self.previous_unnamed_list_tags,
        )
    }

    pub fn start_unnamed_compound_tags(
        &mut self,
        depth: usize,
    ) -> ContiguousTagsAllocator<NbtCompound<'a>> {
        start_allocating_tags_with_depth(
            depth,
            &mut self.unnamed_compound_tags,
            &mut self.previous_unnamed_compound_tags,
        )
    }
    pub fn finish_unnamed_compound_tags(
        &mut self,
        alloc: ContiguousTagsAllocator<NbtCompound<'a>>,
        depth: usize,
    ) -> &'a [NbtCompound<'a>] {
        finish_allocating_tags_with_depth(
            alloc,
            depth,
            &mut self.unnamed_compound_tags,
            &mut self.previous_unnamed_compound_tags,
        )
    }
}
impl Drop for TagAllocator<'_> {
    fn drop(&mut self) {
        self.named_tags
            .iter_mut()
            .for_each(TagsAllocation::deallocate);
        self.previous_named_tags
            .iter_mut()
            .flatten()
            .for_each(TagsAllocation::deallocate);

        self.unnamed_list_tags
            .iter_mut()
            .for_each(TagsAllocation::deallocate);
        self.previous_unnamed_list_tags
            .iter_mut()
            .flatten()
            .for_each(TagsAllocation::deallocate);

        self.unnamed_compound_tags
            .iter_mut()
            .for_each(TagsAllocation::deallocate);
        self.previous_unnamed_compound_tags
            .iter_mut()
            .flatten()
            .for_each(TagsAllocation::deallocate);
    }
}

pub fn start_allocating_tags_with_depth<T>(
    depth: usize,
    tags: &mut Vec<TagsAllocation<T>>,
    previous_allocs: &mut Vec<Vec<TagsAllocation<T>>>,
) -> ContiguousTagsAllocator<T>
where
    T: Clone,
{
    // make sure we have enough space for this depth
    // (also note that depth is reused for compounds and arrays so we might have to push
    // more than once)
    for _ in tags.len()..=depth {
        tags.push(Default::default());
        previous_allocs.push(Default::default());
    }

    let alloc = tags[depth].clone();

    start_allocating_tags(alloc)
}
fn finish_allocating_tags_with_depth<'a, T>(
    alloc: ContiguousTagsAllocator<T>,
    depth: usize,
    tags: &mut [TagsAllocation<T>],
    previous_allocs: &mut [Vec<TagsAllocation<T>>],
) -> &'a [T]
where
    T: Clone,
{
    finish_allocating_tags(alloc, &mut tags[depth], &mut previous_allocs[depth])
}

fn start_allocating_tags<T>(alloc: TagsAllocation<T>) -> ContiguousTagsAllocator<T> {
    let is_new_allocation = alloc.cap == 0;
    ContiguousTagsAllocator {
        alloc,
        is_new_allocation,
        size: 0,
    }
}
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
