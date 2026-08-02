//! FFI-safe rendered graphic frame for non-GTK display surfaces.

/// An FFI-safe rendered graphic frame for non-GTK display surfaces.
///
/// Used by the `GraphicRenderer` trait for headless instances (e.g. MacroPad devices).
/// The caller owns the pixel buffer and must free it via `free`.
#[repr(C)]
pub struct FfiGraphic {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw RGBA pixel data (width * height * 4 bytes).
    pub pixels: *mut u8,
    /// Length of the pixel buffer in bytes.
    pub pixels_len: usize,
}

impl FfiGraphic {
    /// Create an `FfiGraphic` from a width, height, and owned pixel vector.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let pixels_len = pixels.len();
        let mut boxed = pixels.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        Self {
            width,
            height,
            pixels: ptr,
            pixels_len,
        }
    }

    /// Returns the pixel data as a slice. The caller must not free the buffer
    /// while this slice is in use.
    pub fn as_pixels(&self) -> &[u8] {
        if self.pixels.is_null() || self.pixels_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.pixels, self.pixels_len) }
        }
    }

    /// Frees the pixel buffer. Must be called exactly once when the graphic
    /// is no longer needed.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other references to the pixel buffer exist.
    pub unsafe fn free(&mut self) {
        if !self.pixels.is_null() && self.pixels_len > 0 {
            unsafe {
                let _ = Vec::from_raw_parts(self.pixels, self.pixels_len, self.pixels_len);
            }
            self.pixels = std::ptr::null_mut();
            self.pixels_len = 0;
        }
    }

    /// Creates a null graphic (zero pixels).
    pub fn null() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: std::ptr::null_mut(),
            pixels_len: 0,
        }
    }
}

impl Drop for FfiGraphic {
    fn drop(&mut self) {
        unsafe { self.free() }
    }
}
