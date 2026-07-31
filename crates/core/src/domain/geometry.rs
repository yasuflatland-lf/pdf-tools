const LATTICE_PT: f32 = 1.0;

/// The dimensions of a page in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

/// A page-size equivalence class on the one-point lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeClass {
    width_cells: i32,
    height_cells: i32,
}

impl PageSize {
    /// The dimensions of an A4 page in portrait orientation.
    pub const A4_PORTRAIT: PageSize = PageSize {
        width_pt: 595.276,
        height_pt: 841.89,
    };

    /// Returns the page's equivalence class on the one-point lattice.
    pub fn size_class(&self) -> SizeClass {
        SizeClass {
            width_cells: (self.width_pt / LATTICE_PT).round() as i32,
            height_cells: (self.height_pt / LATTICE_PT).round() as i32,
        }
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
    fn page_sizes_in_the_same_lattice_cell_have_the_same_size_class() {
        let a = PageSize {
            width_pt: 595.2,
            height_pt: 841.2,
        };
        let b = PageSize {
            width_pt: 595.4,
            height_pt: 841.4,
        };
        let c = PageSize {
            width_pt: 595.5,
            height_pt: 841.5,
        };
        assert_eq!(a.size_class(), b.size_class());
        assert_ne!(a.size_class(), c.size_class());
    }
}
