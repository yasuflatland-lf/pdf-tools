use std::path::{Path, PathBuf};

use super::geometry::PageSize;
use super::ids::SourceId;

/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Pdf,
    Image,
}

impl SourceKind {
    /// Classifies a path by its extension, case-insensitively. Returns `None`
    /// for anything this app does not merge, so a caller can skip such a path
    /// without a special case. Both adding files and expanding a folder ask
    /// this question, and they must not be able to disagree.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;

        if extension.eq_ignore_ascii_case("pdf") {
            Some(Self::Pdf)
        } else if ["jpg", "jpeg", "png", "gif"]
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        {
            Some(Self::Image)
        } else {
            None
        }
    }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn pdf_and_image_extensions_are_classified() {
        assert_eq!(
            SourceKind::from_extension(Path::new("/a/report.pdf")),
            Some(SourceKind::Pdf)
        );
        for name in ["photo.jpg", "photo.jpeg", "photo.png", "photo.gif"] {
            assert_eq!(
                SourceKind::from_extension(Path::new(name)),
                Some(SourceKind::Image),
                "{name} should classify as an image"
            );
        }
    }

    #[test]
    fn extensions_are_matched_case_insensitively() {
        assert_eq!(
            SourceKind::from_extension(Path::new("/a/UPPER.PDF")),
            Some(SourceKind::Pdf)
        );
        assert_eq!(
            SourceKind::from_extension(Path::new("/a/Photo.JpEg")),
            Some(SourceKind::Image)
        );
    }

    #[test]
    fn anything_else_is_unclassified() {
        for name in ["notes.txt", "archive.pdf.zip", "no-extension", "/a/.hidden"] {
            assert_eq!(
                SourceKind::from_extension(Path::new(name)),
                None,
                "{name} should not classify"
            );
        }
    }
}
