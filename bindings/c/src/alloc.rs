use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    ffi::c_void,
    ptr::NonNull,
};

const GLOBAL: Global = Global;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct w2s_allocator_layout {
    size: usize,
    log2_align: u16,
}

impl w2s_allocator_layout {
    #[inline]
    pub fn from_rust_layout(layout: &Layout) -> Self {
        return Self {
            size: layout.size(),
            log2_align: unsafe { layout.align().checked_ilog2().unwrap_unchecked() as u16 },
        };
    }

    #[inline]
    pub unsafe fn to_rust_layout(self) -> Layout {
        return Layout::from_size_align_unchecked(self.size, 1 << self.log2_align);
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct w2s_allocator {
    alloc: Option<unsafe extern "C" fn(w2s_allocator_layout) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, w2s_allocator_layout)>,
}

impl Allocator for w2s_allocator {
    fn allocate(
        &self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
        unsafe {
            if let Some(alloc) = self.alloc {
                let ptr = (alloc)(w2s_allocator_layout::from_rust_layout(&layout));
                if ptr.is_null() {
                    return Err(AllocError);
                }

                let bytes = core::slice::from_raw_parts_mut(ptr.cast(), layout.size());
                return Ok(NonNull::new_unchecked(bytes));
            }
        }
        return GLOBAL.allocate(layout);
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: std::alloc::Layout) {
        if let Some(free) = self.free {
            return (free)(
                ptr.as_ptr().cast(),
                w2s_allocator_layout::from_rust_layout(&layout),
            );
        }
        return GLOBAL.deallocate(ptr, layout);
    }
}
