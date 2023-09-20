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
    ptr: NonNull<Pbuf>,
}

impl PbufPtr {
    /// Takes ownership of a pointer to a [`Pbuf`].
    /// Does not adjust reference counts.
    pub unsafe fn new(ptr: NonNull<Pbuf>) -> Self {
        PbufPtr { ptr }
    }

    /// Takes ownership of a pointer to a [`sys::pbuf`].
    /// Does not adjust reference counts.
    pub unsafe fn new_from_ffi(ptr: NonNull<sys::pbuf>) -> Self {
        Self::new(ptr.cast())
    }

    /// Creates a new reference to a [`Pbuf`]. Increments reference count.
    pub unsafe fn new_ref(ptr: NonNull<Pbuf>) -> Self {
        unsafe { sys::pbuf_ref(ptr.as_ptr().cast()); }
        PbufPtr { ptr }
    }

    /// Creates a new reference to a [`sys::pbuf`].
    /// Increments reference count.
    pub unsafe fn ref_from_ffi(ptr: NonNull<sys::pbuf>) -> Self {
        Self::new_ref(ptr.cast())
    }

    /// Consumes self and returns an owned [`Pbuf`] pointer.
    /// Does not adjust reference counts.
    pub fn into_raw(pbuf: Self) -> NonNull<Pbuf> {
        let ptr = pbuf.ptr;
        mem::forget(pbuf);
        ptr
    }

    /// Returns a *borrowed* pointer to the underlying [`sys::pbuf`] struct.
    /// Pointer only valid for the lifetime of this object.
    pub fn as_ptr(pbuf: &Self) -> *const sys::pbuf {
        pbuf.ptr.as_ptr().cast()
    }

    /// Returns a *borrowed* mutable pointer to the underlying [`sys::pbuf`]
    /// struct. Pointer only valid for the lifetime of this object.
    pub fn as_mut_ptr(pbuf: &Self) -> *mut sys::pbuf {
        pbuf.ptr.as_ptr().cast()
    }
}

impl Deref for PbufPtr {
    type Target = Pbuf;

    fn deref(&self) -> &Pbuf {
        unsafe { self.ptr.as_ref() }
    }
}

impl DerefMut for PbufPtr {
    fn deref_mut(&mut self) -> &mut Pbuf {
        unsafe { self.ptr.as_mut() }
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
