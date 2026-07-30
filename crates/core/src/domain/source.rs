use std::path::PathBuf;

use super::geometry::PageSize;
use super::ids::SourceId;

/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Pdf,
    Image,
}

/// Whether pages from a source are grouped together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grouping {
    Grouped,
    Ungrouped,
}

/// The availability of a source file.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceStatus {
    Ready,
    Encrypted,
    Unreadable { reason: String },
}

/// A source file and its probed metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub kind: SourceKind,
    pub grouping: Grouping,
    pub page_count: u32,
    /// One entry per page for PDF sources. Empty for images: an image page is
    /// sized from the plan's dominant page size, never from the file itself.
    pub page_sizes: Vec<PageSize>,
    pub status: SourceStatus,
}

/// What `PdfEngine::probe` reports about a document.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentInfo {
    pub page_count: u32,
    pub page_sizes: Vec<PageSize>,
    pub encrypted: bool,
}

/// What `ImageDecoder::probe` reports. DPI is deliberately absent: page sizing
/// follows the plan's dominant page size, so image DPI is never consulted.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    pub width_px: u32,
    pub height_px: u32,
}
