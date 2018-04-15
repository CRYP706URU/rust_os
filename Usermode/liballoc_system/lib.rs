//
//
//
#![crate_type="rlib"]
#![crate_name="alloc_system"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(const_fn)]
#![feature(box_syntax)]
#![feature(optin_builtin_traits)]	// For !Send
#![feature(unboxed_closures)]
#![feature(alloc)]
#![feature(allocator_api)]
#![no_std]

use alloc::allocator::{Layout,AllocErr,CannotReallocInPlace};
use alloc::allocator::Opaque;
use core::ptr::NonNull;

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

extern crate std_sync as sync;
extern crate alloc;

mod std {
	pub use core::fmt;
}

mod heap;


pub fn oom() {
	panic!("Out of memory");
}


pub struct Allocator;
pub const ALLOCATOR: &Allocator = &Allocator;

unsafe impl alloc::allocator::Alloc for &'static Allocator
{
	unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<Opaque>, AllocErr>
	{
		let rv = heap::allocate(layout.size(), layout.align());
		if rv == ::core::ptr::null_mut()
		{
			Err(AllocErr)
		}
		else
		{
			Ok(NonNull::new_unchecked(rv as *mut Opaque))
		}
	}
	unsafe fn dealloc(&mut self, ptr: NonNull<Opaque>, layout: Layout)
	{
		heap::deallocate(ptr.as_ptr() as *mut u8, layout.size(), layout.align())
	}
	unsafe fn realloc(&mut self, ptr: NonNull<Opaque>, layout: Layout, new_size: usize) -> Result<NonNull<Opaque>, AllocErr>
	{
		let rv = heap::reallocate(ptr.as_ptr() as *mut u8, layout.size(), layout.align(), new_size);
		if rv == ::core::ptr::null_mut()
		{
			Err(AllocErr)
		}
		else
		{
			Ok(NonNull::new_unchecked(rv as *mut Opaque))
		}
	}
	// TODO: alloc_excess
	unsafe fn grow_in_place(&mut self, ptr: NonNull<Opaque>, layout: Layout, new_size: usize) -> Result<(), CannotReallocInPlace>
	{
		let rv = heap::reallocate_inplace(ptr.as_ptr() as *mut u8, layout.size(), layout.align(), new_size);
		if rv != new_size
		{
			Err(CannotReallocInPlace)
		}
		else
		{
			Ok( () )
		}
	}
	// TODO: shrink_in_place
}
