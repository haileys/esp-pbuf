#![no_std]

use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};
use core::slice;

pub mod sys;
pub mod raw;

use raw::PbufPtr;

#[repr(transparent)]
/// A safe wrapper around the underlying [`sys::pbuf`] struct. This struct is
/// usable by reference - you will never have an owned copy of it. See
/// [`PbufRef`] for an owned reference.
pub struct Pbuf {
    raw: sys::pbuf,
}

unsafe impl Send for Pbuf {}
unsafe impl Sync for Pbuf {}

impl Pbuf {
    pub fn from_ref(pbuf: &sys::pbuf) -> &Pbuf {
        // SAFETY: Pbuf is repr(transparent)
        unsafe { mem::transmute(pbuf) }
    }

    pub fn from_mut_ref(pbuf: &mut sys::pbuf) -> &mut Pbuf {
        // SAFETY: Pbuf is repr(transparent)
        unsafe { mem::transmute(pbuf) }
    }

    /// The length of this buffer. Does not include other buffers in chain.
    pub fn len(&self) -> usize {
        self.raw.len.into()
    }

    pub fn bytes_ptr(&self) -> *const u8 {
        self.raw.payload.cast()
    }

    pub fn bytes_mut_ptr(&mut self) -> *mut u8 {
        self.raw.payload.cast()
    }

    pub fn bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.bytes_ptr(), self.len()) }
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.bytes_mut_ptr(), self.len()) }
    }

    /// The next buffer in the chain.
    pub fn next(&self) -> Option<&Pbuf> {
        NonNull::new(self.raw.next).map(|ptr| {
            unsafe { ptr.cast::<Pbuf>().as_ref() }
        })
    }
}

/// A shared reference to [`Pbuf`]. Reference counted, immutable.
/// See [`PbufMut`] for a uniquely owned + mutable reference.
#[repr(transparent)]
pub struct PbufRef {
    ptr: PbufPtr,
}

impl PbufRef {
    /// Construct a `Pbuf` from a raw [`PbufPtr`]
    pub fn from_ptr(ptr: PbufPtr) -> Self {
        PbufRef { ptr }
    }
}

impl Deref for PbufRef {
    type Target = Pbuf;

    fn deref(&self) -> &Pbuf {
        &self.ptr
    }
}

pub struct PbufMut {
    ptr: PbufPtr,
}

impl PbufMut {
    /// Try to construct a `PbufMut` from a raw [`PbufPtr`]. If the pointer
    /// is not uniquely owned, returns a [`PbufRef`] in the error variant.
    pub fn try_from_ptr(ptr: PbufPtr) -> Result<PbufMut, PbufRef> {
        if ptr.raw.ref_ == 1 {
            Ok(PbufMut { ptr })
        } else {
            Err(PbufRef { ptr })
        }
    }
}

impl Deref for PbufMut {
    type Target = Pbuf;

    fn deref(&self) -> &Pbuf {
        &self.ptr
    }
}

impl DerefMut for PbufMut {
    fn deref_mut(&mut self) -> &mut Pbuf {
        &mut self.ptr
    }
}

pub struct PbufUninit {
    ptr: PbufPtr,
}

#[derive(Debug, Clone, Copy)]
pub struct AllocatePbufError;

impl PbufUninit {
    pub fn allocate(layer: sys::pbuf_layer, length: usize, type_: sys::pbuf_type)
        -> Result<Self, AllocatePbufError>
    {
        let length = u16::try_from(length).map_err(|_| AllocatePbufError)?;
        let ptr = unsafe { sys::pbuf_alloc(layer, length, type_) };
        let ptr = NonNull::new(ptr).ok_or(AllocatePbufError)?;
        let ptr = unsafe { PbufPtr::new(ptr) };
        Ok(PbufUninit { ptr })
    }

    pub fn zeroed(mut self) -> PbufMut {
        unsafe {
            ptr::write_bytes(self.bytes_mut_ptr(), 0, self.ptr.len());
            self.assume_init()
        }
    }

    pub fn len(&self) -> usize {
        self.ptr.len()
    }

    pub fn bytes_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.bytes_mut_ptr()
    }

    pub unsafe fn assume_init(self) -> PbufMut {
        PbufMut { ptr: self.ptr }
    }
}
