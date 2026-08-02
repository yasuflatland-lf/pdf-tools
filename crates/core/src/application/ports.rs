use std::path::{Path, PathBuf};

use crate::application::errors::{ImageError, PdfError, WalkError};
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

/// One entry inside a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkEntry {
    pub path: PathBuf,
    /// A symlink reports `false` even when it points at a directory. The walk
    /// only descends where this is `true`, so a link cycle cannot hang a scan.
    pub is_dir: bool,
}

/// Lists one directory at a time, and nothing more.
///
/// The traversal itself lives in the application layer rather than here. That
/// is what keeps walk order, extension filtering and hidden-entry rules
/// testable without touching a filesystem, and it leaves this port with a
/// surface small enough that a fake is a plain map.
pub trait DirectoryWalker: Send + Sync {
    /// Whether the path is a directory the walk may start from. Unlike
    /// `WalkEntry::is_dir` this follows symlinks: a folder the user picked
    /// deliberately should work even when it is a link.
    fn is_dir(&self, path: &Path) -> bool;

    fn read_dir(&self, dir: &Path) -> Result<Vec<WalkEntry>, WalkError>;
}
