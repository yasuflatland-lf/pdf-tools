/// The dimensions of a page in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

impl PageSize {
    /// The dimensions of an A4 page in portrait orientation.
    pub const A4_PORTRAIT: PageSize = PageSize {
        width_pt: 595.276,
        height_pt: 841.89,
    };

    /// Two page sizes match when both dimensions differ by less than 1 pt.
    pub fn approx_eq(&self, other: &PageSize) -> bool {
        (self.width_pt - other.width_pt).abs() < 1.0
            && (self.height_pt - other.height_pt).abs() < 1.0
    }
}

/// The requested dimensions for rasterizing a page.
#[derive(Debug, Clone, Copy)]
pub struct RasterSpec {
    pub target_width_px: u32,
}

/// An RGBA raster image.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_sizes_within_one_point_are_the_same_size() {
        let a = PageSize {
            width_pt: 595.276,
            height_pt: 841.89,
        };
        let b = PageSize {
            width_pt: 595.9,
            height_pt: 841.2,
        };
        let c = PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        }; // US Letter
        assert!(a.approx_eq(&b));
        assert!(!a.approx_eq(&c));
    }
}
