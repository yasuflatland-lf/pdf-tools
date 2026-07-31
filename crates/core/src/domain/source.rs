use std::path::PathBuf;

use super::geometry::PageSize;
use super::ids::SourceId;

/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Pdf,
    Image,
}

/// Why a source file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreadableReason {
    /// Not a format this app decodes.
    UnsupportedFormat,
    /// The file exists but its contents could not be parsed.
    Damaged,
    /// The file disappeared between being added and being read.
    Missing,
    /// The engine itself was unavailable.
    EngineUnavailable,
}

/// The availability of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatus {
    Ready,
    Encrypted,
    Unreadable(UnreadableReason),
}

/// A source file and its probed metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub kind: SourceKind,
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
