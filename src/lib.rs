#![no_std]

use core::alloc::Layout;
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
pub enum AllocatePbufError {
    /// pbuf uses u16 lengths internally, we return this error if the
    /// requested packet length is too large:
    LengthLargerThanU16,
    /// pbuf_alloc returned null:
    AllocationFailed,
}

impl PbufUninit {
    pub fn allocate(layer: sys::pbuf_layer, type_: sys::pbuf_type, length: usize)
        -> Result<Self, AllocatePbufError>
    {
        let length = u16::try_from(length)
            .map_err(|_| AllocatePbufError::LengthLargerThanU16)?;

        let ptr = unsafe { sys::pbuf_alloc(layer, length, type_) };

        let ptr = NonNull::new(ptr)
            .ok_or(AllocatePbufError::AllocationFailed)?;

        let ptr = unsafe { PbufPtr::new(ptr) };
        Ok(PbufUninit { ptr })
    }

    /// Like [`allocate`], but allows for alignment of payload buffer
    /// to be controlled.
    pub fn allocate_layout(layer: sys::pbuf_layer, type_: sys::pbuf_type, layout: Layout)
        -> Result<Self, AllocatePbufError>
    {
        // pad allocation size to account for worst case:
        let alloc_size = layout.size() + layout.align();

        // allocate the pbuf with the padded allocation size:
        let mut pbuf = Self::allocate(layer, type_, alloc_size)?;

        // figure out how many bytes we need to adjust payload ptr by to
        // achieve requested alignment:
        let adjust = pbuf.bytes_mut_ptr().align_offset(layout.align());

        // adjust the pbuf to requested alignment:
        unsafe {
            let pbuf = PbufPtr::as_mut_ptr(&pbuf.ptr);

            // pbuf_remove_header increments the underlying ptr
            // never fails in this use:
            assert!(sys::pbuf_remove_header(pbuf, adjust) == 0);

            // pbuf_realloc shrinks the underlying length fields without
            // actually calling into the allocator:
            sys::pbuf_realloc(pbuf, layout.size() as u16);
        }

        Ok(pbuf)
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

    /// Initializes the pbuf from slice. Like `slice::copy_from_slice`, panics
    /// on mismatched length:
    pub fn copied_from_slice(mut self, slice: &[u8]) -> PbufMut {
        assert!(slice.len() == self.len());
        unsafe {
            ptr::copy(slice.as_ptr(), self.bytes_mut_ptr(), self.len());
            self.assume_init()
        }
    }

    pub fn bytes_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.bytes_mut_ptr()
    }

    pub unsafe fn assume_init(self) -> PbufMut {
        PbufMut { ptr: self.ptr }
    }
}
