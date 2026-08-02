const LATTICE_PT: f32 = 1.0;

/// The dimensions of a page in points, before any rotation. A slot's rotation
/// is written as the page's `/Rotate` attribute rather than by exchanging
/// these axes, so this is the sheet a page is built on and not the shape it is
/// seen as.
#[derive(Debug, Clone, Copy)]
pub struct PageSize {
    width_pt: f32,
    height_pt: f32,
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

    /// Creates a page size, or `None` when either dimension is not a positive
    /// finite number of points. A page with no area cannot be composed onto:
    /// fitting an image to it divides by zero.
    pub fn new(width_pt: f32, height_pt: f32) -> Option<Self> {
        (width_pt.is_finite() && height_pt.is_finite() && width_pt > 0.0 && height_pt > 0.0)
            .then_some(Self {
                width_pt,
                height_pt,
            })
    }

    pub fn width_pt(self) -> f32 {
        self.width_pt
    }

    pub fn height_pt(self) -> f32 {
        self.height_pt
    }

    /// Returns the page size with its width and height exchanged.
    pub fn turned(self) -> Self {
        Self {
            width_pt: self.height_pt,
            height_pt: self.width_pt,
        }
    }

    /// Returns the page's equivalence class on the one-point lattice.
    pub fn size_class(&self) -> SizeClass {
        SizeClass {
            width_cells: (self.width_pt / LATTICE_PT).round() as i32,
            height_cells: (self.height_pt / LATTICE_PT).round() as i32,
        }
    }
}

/// Two pages are the same size when they land in the same cell of the
/// one-point lattice. Raw float equality would make 595.2 pt and 595.4 pt
/// different pages, which is a distinction no reader of the output can make
/// and one that `dominant_page_size` deliberately does not draw.
impl PartialEq for PageSize {
    fn eq(&self, other: &Self) -> bool {
        self.size_class() == other.size_class()
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
    fn turned_page_size_exchanges_the_axes() {
        let size = PageSize::new(612.0, 792.0).expect("page size should be valid");

        assert_eq!(
            size.turned(),
            PageSize::new(792.0, 612.0).expect("turned page size should be valid")
        );
    }

    #[test]
    fn new_accepts_positive_finite_dimensions() {
        assert_eq!(PageSize::new(595.276, 841.89), Some(PageSize::A4_PORTRAIT));
    }

    #[test]
    fn new_rejects_dimensions_that_are_not_positive_and_finite() {
        for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(PageSize::new(invalid, 841.89), None);
            assert_eq!(PageSize::new(595.276, invalid), None);
        }
    }

    #[test]
    fn page_size_equality_uses_the_one_point_lattice() {
        let a = PageSize::new(595.2, 841.2).expect("page size should be valid");
        let b = PageSize::new(595.4, 841.4).expect("page size should be valid");
        let c = PageSize::new(595.5, 841.5).expect("page size should be valid");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
