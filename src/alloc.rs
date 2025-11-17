use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use crate::str::Str;
use crate::vm::{Value, Object};
use crate::ast::ClassId;

pub struct Alloc
{
    mem_block: *mut u8,
    mem_size: usize,
    next_idx: usize,
}

impl Alloc
{
    pub fn new() -> Self
    {
        let mem_size = 256 * 1024 * 1024;
        let layout = Layout::from_size_align(mem_size, 8).unwrap();

        let mem_block = unsafe { alloc_zeroed(layout) };
        if mem_block.is_null() {
            panic!();
        }

        Self {
            mem_block,
            mem_size,
            next_idx: 0,
        }
    }

    // Allocate a block of a given size
    fn alloc_bytes(&mut self, size_bytes: usize) -> *mut u8
    {
        let align_bytes = 8;

        // Align the current alloc index
        let obj_pos = (self.next_idx + (align_bytes - 1)) & !(align_bytes - 1);

        // Bump the next allocation index
        let next_idx = obj_pos + size_bytes;
        if next_idx >= self.mem_size {
            panic!("allocator out of memory");
        }
        self.next_idx = next_idx;

        unsafe {
            self.mem_block.add(obj_pos)
        }
    }

    // Allocate a variable-sized table of elements of a given type
    pub fn alloc_table<T>(&mut self, num_elems: usize) -> *mut [T]
    {
        let num_bytes = num_elems * std::mem::size_of::<T>();
        let bytes = self.alloc_bytes(num_bytes);
        let p = bytes as *mut T;

        std::ptr::slice_from_raw_parts_mut(p, num_elems)
    }

    // Allocate a new object with a given number of slots
    pub fn new_object(&mut self, class_id: ClassId, num_slots: usize) -> Value
    {
        // Allocate the slots for the object
        let slots = self.alloc_table::<Value>(num_slots);

        // Create the Object struct
        let obj = Object::new(class_id, slots);

        // Allocate the Object struct itself
        let obj_ptr = self.alloc(obj);

        Value::Object(obj_ptr)
    }

    // Allocate a new object of a given type
    pub fn alloc<T>(&mut self, obj: T) -> *mut T
    {
        let num_bytes = std::mem::size_of::<T>();
        let bytes = self.alloc_bytes(num_bytes);
        let p = bytes as *mut T;

        // Write object at location without calling drop
        // on what's currently at that location
        unsafe { std::ptr::write(p, obj) };

        p
    }

    pub fn str(&mut self, s: &str) -> *const Str
    {
        let bytes = self.alloc_bytes(s.len());
        let p = bytes as *mut u8;

        // Write string bytes at location without calling drop
        // on what's currently at that location
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len()) };
        let raw_str = unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, s.len()))
        };
        let raw_str_ptr = raw_str as *const str;
        let str = self.alloc(Str::new(raw_str_ptr));
        str
    }

    pub fn str_val(&mut self, s: &str) -> Value
    {
        Value::String(self.str(s))
    }
}

// Allow sending allocators between threads
// This is needed for the message allocator
unsafe impl Send for Alloc {}
unsafe impl Sync for Alloc {}
