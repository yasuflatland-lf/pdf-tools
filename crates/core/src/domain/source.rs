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
    const PDF_EXTENSIONS: [&'static str; 1] = ["pdf"];
    const IMAGE_EXTENSIONS: [&'static str; 4] = ["jpg", "jpeg", "png", "gif"];

    /// Classifies a path by its extension, case-insensitively. Returns `None`
    /// for anything this app does not merge, so a caller can skip such a path
    /// without a special case. Adding files, expanding a folder, decoding an
    /// image and filtering the picker all ask this question here, and so they
    /// cannot disagree.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;

        if matches(&Self::PDF_EXTENSIONS, extension) {
            Some(Self::Pdf)
        } else if matches(&Self::IMAGE_EXTENSIONS, extension) {
            Some(Self::Image)
        } else {
            None
        }
    }

    /// Every extension this app merges, for a file picker's filter.
    pub fn supported_extensions() -> Vec<&'static str> {
        Self::PDF_EXTENSIONS
            .iter()
            .chain(Self::IMAGE_EXTENSIONS.iter())
            .copied()
            .collect()
    }
}

fn matches(candidates: &[&str], extension: &str) -> bool {
    candidates
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
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
    id: SourceId,
    path: PathBuf,
    kind: SourceKind,
    page_count: u32,
    /// One entry per page for PDF sources. Empty for images: an image page is
    /// sized from the plan's dominant page size, never from the file itself.
    page_sizes: Vec<PageSize>,
    status: SourceStatus,
}

impl SourceFile {
    /// A PDF whose pages were probed successfully. `page_count` is taken from
    /// the sizes rather than passed alongside them, so the two cannot disagree.
    pub fn ready_pdf(id: SourceId, path: PathBuf, page_sizes: Vec<PageSize>) -> Self {
        Self {
            id,
            path,
            kind: SourceKind::Pdf,
            page_count: page_sizes.len() as u32,
            page_sizes,
            status: SourceStatus::Ready,
        }
    }

    /// An image that decoded. Every image is exactly one page, and its size
    /// comes from the plan's dominant page size rather than from the file.
    pub fn ready_image(id: SourceId, path: PathBuf) -> Self {
        Self {
            id,
            path,
            kind: SourceKind::Image,
            page_count: 1,
            page_sizes: Vec::new(),
            status: SourceStatus::Ready,
        }
    }

    /// A file the app will show but not merge. It contributes no slots, so it
    /// has no pages and no page sizes -- the one shape every failure takes.
    pub fn failed(id: SourceId, path: PathBuf, kind: SourceKind, status: SourceStatus) -> Self {
        debug_assert!(
            status != SourceStatus::Ready,
            "a failed source cannot be Ready"
        );
        Self {
            id,
            path,
            kind,
            page_count: 0,
            page_sizes: Vec::new(),
            status,
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> SourceKind {
        self.kind
    }

    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn page_sizes(&self) -> &[PageSize] {
        &self.page_sizes
    }

    pub fn status(&self) -> SourceStatus {
        self.status
    }
}

/// What `PdfEngine::probe` reports about a document.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentInfo {
    /// One entry per page. The page count is read off this rather than stored
    /// beside it, so the two cannot disagree.
    pub page_sizes: Vec<PageSize>,
    pub encrypted: bool,
}

impl DocumentInfo {
    pub fn page_count(&self) -> u32 {
        self.page_sizes.len() as u32
    }
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

    #[test]
    fn supported_extensions_lists_every_mergeable_format() {
        assert_eq!(
            SourceKind::supported_extensions(),
            ["pdf", "jpg", "jpeg", "png", "gif"]
        );
    }

    #[test]
    fn ready_pdf_derives_page_count_from_the_sizes() {
        let page_sizes = vec![PageSize::A4_PORTRAIT; 2];
        let source = SourceFile::ready_pdf(SourceId(7), "/a.pdf".into(), page_sizes.clone());

        assert_eq!(source.id(), SourceId(7));
        assert_eq!(source.path(), Path::new("/a.pdf"));
        assert_eq!(source.kind(), SourceKind::Pdf);
        assert_eq!(source.page_count(), 2);
        assert_eq!(source.page_sizes(), page_sizes);
        assert_eq!(source.status(), SourceStatus::Ready);
    }

    #[test]
    fn page_count_follows_the_page_sizes() {
        let three_pages = DocumentInfo {
            page_sizes: vec![PageSize::A4_PORTRAIT; 3],
            encrypted: false,
        };
        let no_pages = DocumentInfo {
            page_sizes: Vec::new(),
            encrypted: false,
        };

        assert_eq!(three_pages.page_count(), 3);
        assert_eq!(no_pages.page_count(), 0);
    }

    #[test]
    fn ready_image_reports_one_page_and_no_sizes() {
        let source = SourceFile::ready_image(SourceId(7), "/a.png".into());

        assert_eq!(source.id(), SourceId(7));
        assert_eq!(source.path(), Path::new("/a.png"));
        assert_eq!(source.kind(), SourceKind::Image);
        assert_eq!(source.page_count(), 1);
        assert!(source.page_sizes().is_empty());
        assert_eq!(source.status(), SourceStatus::Ready);
    }

    #[test]
    fn failed_reports_no_pages_or_sizes_for_every_failure_status() {
        let statuses = [
            SourceStatus::Encrypted,
            SourceStatus::Unreadable(UnreadableReason::UnsupportedFormat),
            SourceStatus::Unreadable(UnreadableReason::Damaged),
            SourceStatus::Unreadable(UnreadableReason::Missing),
            SourceStatus::Unreadable(UnreadableReason::EngineUnavailable),
        ];

        for status in statuses {
            let source = SourceFile::failed(SourceId(7), "/a.pdf".into(), SourceKind::Pdf, status);

            assert_eq!(source.page_count(), 0);
            assert!(source.page_sizes().is_empty());
            assert_eq!(source.status(), status);
        }
    }
}
