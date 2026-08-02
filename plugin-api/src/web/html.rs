//! FFI-safe HTML string for passing HTML fragments across the plugin boundary.

/// An FFI-safe string for passing HTML fragments across the plugin boundary.
///
/// Used by the `WebRenderer` trait for web instances.
/// The caller owns the string data and must free it via `free`.
#[repr(C)]
pub struct FfiHtmlString {
    /// UTF-8 string data.
    pub data: *mut u8,
    /// Length of the string in bytes.
    pub len: usize,
}

impl FfiHtmlString {
    /// Create an `FfiHtmlString` from an owned `String`.
    pub fn from_string(s: String) -> Self {
        let len = s.len();
        let mut boxed = s.into_bytes().into_boxed_slice();
        let data = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        Self { data, len }
    }

    /// Returns the string content as a `&str`. The caller must not free the
    /// buffer while this reference is in use.
    pub fn as_str(&self) -> &str {
        if self.data.is_null() || self.len == 0 {
            ""
        } else {
            unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data, self.len)) }
        }
    }

    /// Frees the string buffer. Must be called exactly once when the string
    /// is no longer needed.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other references to the string buffer exist.
    pub unsafe fn free(&mut self) {
        if !self.data.is_null() && self.len > 0 {
            unsafe {
                let _ = Vec::from_raw_parts(self.data, self.len, self.len);
            }
            self.data = std::ptr::null_mut();
            self.len = 0;
        }
    }

    /// Creates a null string (empty).
    pub fn null() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

impl Drop for FfiHtmlString {
    fn drop(&mut self) {
        unsafe { self.free() }
    }
}
