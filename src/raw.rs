use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::sys;
use crate::Pbuf;

/// Represents an raw owned pointer to a pbuf.
/// Handles reference counting and freeing on drop automatically.
/// This is the building block for the safe wrapper types [`PbufRef`] and
/// [`PbufMut`].
#[repr(transparent)]
pub struct PbufPtr {
    ptr: NonNull<sys::pbuf>,
}

impl PbufPtr {
    /// Takes ownership of a pointer to a [`sys::pbuf`].
    /// Does not adjust reference counts.
    pub unsafe fn new(ptr: NonNull<sys::pbuf>) -> Self {
        PbufPtr { ptr: ptr.cast() }
    }
    /// Creates a new reference to a [`sys::pbuf`].
    /// Increments reference count.
    pub unsafe fn new_ref(ptr: NonNull<sys::pbuf>) -> Self {
        unsafe { sys::pbuf_ref(ptr.as_ptr()); }
        PbufPtr { ptr: ptr.cast() }
    }

    /// Consumes self and returns an owned [`sys::pbuf`] pointer.
    /// Does not adjust reference counts.
    pub fn into_raw(pbuf: Self) -> NonNull<sys::pbuf> {
        let ptr = pbuf.ptr;
        mem::forget(pbuf);
        ptr
    }

    /// Returns a *borrowed* pointer to the underlying [`sys::pbuf`] struct.
    /// Pointer only valid for the lifetime of this object.
    pub fn as_ptr(pbuf: &Self) -> *const sys::pbuf {
        pbuf.ptr.as_ptr()
    }

    /// Returns a *borrowed* mutable pointer to the underlying [`sys::pbuf`]
    /// struct. Pointer only valid for the lifetime of this object.
    pub fn as_mut_ptr(pbuf: &Self) -> *mut sys::pbuf {
        pbuf.ptr.as_ptr()
    }
}

impl Deref for PbufPtr {
    type Target = Pbuf;

    fn deref(&self) -> &Pbuf {
        unsafe { self.ptr.cast().as_ref() }
    }
}

impl DerefMut for PbufPtr {
    fn deref_mut(&mut self) -> &mut Pbuf {
        unsafe { self.ptr.cast().as_mut() }
    }
}

impl Clone for PbufPtr {
    fn clone(&self) -> Self {
        unsafe { PbufPtr::new_ref(self.ptr) }
    }
}

impl Drop for PbufPtr {
    fn drop(&mut self) {
        unsafe { sys::pbuf_free(PbufPtr::as_mut_ptr(self)); }
    }
}
