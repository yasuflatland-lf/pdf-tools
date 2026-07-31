use std::path::{Path, PathBuf};

use crate::application::errors::{ImageError, PdfError};
use crate::domain::geometry::{PageSize, RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use crate::domain::plan::Rotation;
use crate::domain::source::{DocumentInfo, ImageInfo};

/// A fully resolved merge instruction.
///
/// `MergePlan` holds only source identifiers, so the application layer resolves
/// them to paths before handing work to the engine. This keeps the engine free
/// of any knowledge about sources and grouping.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposePlan {
    pub entries: Vec<ComposeEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComposeEntry {
    PdfPage {
        path: PathBuf,
        page: PageIndex,
        rotation: Rotation,
    },
    Image {
        path: PathBuf,
        fit_to: PageSize,
        rotation: Rotation,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeReport {
    pub page_count: u32,
    pub bytes_written: u64,
}

pub trait PdfEngine: Send + Sync {
    fn probe(&self, src: &Path) -> Result<DocumentInfo, PdfError>;

    fn rasterize(
        &self,
        src: &Path,
        page: PageIndex,
        spec: RasterSpec,
    ) -> Result<RasterImage, PdfError>;

    fn compose(&self, plan: &ComposePlan, dest: &Path) -> Result<MergeReport, PdfError>;
}

pub trait ImageDecoder: Send + Sync {
    fn probe(&self, src: &Path) -> Result<ImageInfo, ImageError>;

    fn decode_first_frame(&self, src: &Path) -> Result<RasterImage, ImageError>;
}
