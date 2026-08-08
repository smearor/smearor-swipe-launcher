/// A buffer of RGBA pixel data (4 bytes per pixel: red, green, blue, alpha).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaPixels(Vec<u8>);

impl RgbaPixels {
    pub fn new(pixels: Vec<u8>) -> Self {
        Self(pixels)
    }

    /// Extract a rectangular grid slice from an RGBA pixel buffer.
    ///
    /// Crops the region (x_offset, y_offset) to (slice_width, slice_height)
    /// from the source buffer.
    pub fn extract_grid_slice(pixels: &[u8], src_width: u32, src_height: u32, x_offset: u32, y_offset: u32, slice_width: u32, slice_height: u32) -> Self {
        let _ = src_height;
        let mut result = Vec::with_capacity((slice_width * slice_height * 4) as usize);
        for y in y_offset..(y_offset + slice_height) {
            let start = ((y * src_width + x_offset) * 4) as usize;
            let end = start + (slice_width * 4) as usize;
            result.extend_from_slice(&pixels[start..end]);
        }
        Self(result)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u8>> for RgbaPixels {
    fn from(pixels: Vec<u8>) -> Self {
        Self(pixels)
    }
}

impl From<RgbaPixels> for Vec<u8> {
    fn from(pixels: RgbaPixels) -> Self {
        pixels.0
    }
}
