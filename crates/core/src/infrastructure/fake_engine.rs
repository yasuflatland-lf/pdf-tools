use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::application::errors::{ImageError, PdfError, WalkError};
use crate::application::ports::{
    ComposePlan, DirectoryWalker, ImageDecoder, MergeReport, PdfEngine, WalkEntry,
};
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use crate::domain::source::{DocumentInfo, ImageInfo};

pub struct FakePdfEngine {
    documents: HashMap<PathBuf, Result<DocumentInfo, PdfError>>,
    last_composed: Mutex<Option<ComposePlan>>,
    last_rasterized: Mutex<Option<(PathBuf, PageIndex, u32)>>,
}

impl FakePdfEngine {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            last_composed: Mutex::new(None),
            last_rasterized: Mutex::new(None),
        }
    }

    pub fn with_document(mut self, path: impl Into<PathBuf>, info: DocumentInfo) -> Self {
        self.documents.insert(path.into(), Ok(info));
        self
    }

    pub fn with_failure(mut self, path: impl Into<PathBuf>, err: PdfError) -> Self {
        self.documents.insert(path.into(), Err(err));
        self
    }

    /// Returns the last compose plan handed to this engine.
    pub fn last_composed(&self) -> Option<ComposePlan> {
        self.last_composed
            .lock()
            .expect("fake PDF engine mutex should not be poisoned")
            .clone()
    }

    /// Returns the last rasterize request handed to this engine as the source
    /// path, the page asked for, and the requested width in pixels. Recorded
    /// even when the request then fails, so a test can assert what was asked.
    pub fn last_rasterized(&self) -> Option<(PathBuf, PageIndex, u32)> {
        self.last_rasterized
            .lock()
            .expect("fake PDF engine mutex should not be poisoned")
            .clone()
    }
}

impl Default for FakePdfEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfEngine for FakePdfEngine {
    fn probe(&self, src: &Path) -> Result<DocumentInfo, PdfError> {
        self.documents.get(src).cloned().unwrap_or_else(|| {
            Err(PdfError::Missing {
                path: src.to_path_buf(),
            })
        })
    }

    fn rasterize(
        &self,
        src: &Path,
        page: PageIndex,
        spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        *self
            .last_rasterized
            .lock()
            .expect("fake PDF engine mutex should not be poisoned") =
            Some((src.to_path_buf(), page, spec.target_width_px));

        let info = self.probe(src)?;
        if page.0 >= info.page_count {
            return Err(PdfError::PageOutOfRange {
                page: page.0,
                count: info.page_count,
            });
        }
        let size = info
            .page_sizes
            .get(page.0 as usize)
            .ok_or(PdfError::PageOutOfRange {
                page: page.0,
                count: info.page_count,
            })?;
        let height =
            ((spec.target_width_px as f32 * size.height_pt) / size.width_pt).round() as u32;
        let pixel_count = spec.target_width_px as usize * height as usize;

        Ok(RasterImage {
            width: spec.target_width_px,
            height,
            rgba: vec![255; pixel_count * 4],
        })
    }

    fn compose(&self, plan: &ComposePlan, _dest: &Path) -> Result<MergeReport, PdfError> {
        *self
            .last_composed
            .lock()
            .expect("fake PDF engine mutex should not be poisoned") = Some(plan.clone());

        Ok(MergeReport {
            page_count: plan.entries.len() as u32,
            bytes_written: 0,
        })
    }
}

pub struct FakeImageDecoder {
    images: HashMap<PathBuf, Result<ImageInfo, ImageError>>,
}

impl FakeImageDecoder {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    pub fn with_image(mut self, path: impl Into<PathBuf>, info: ImageInfo) -> Self {
        self.images.insert(path.into(), Ok(info));
        self
    }

    pub fn with_failure(mut self, path: impl Into<PathBuf>, err: ImageError) -> Self {
        self.images.insert(path.into(), Err(err));
        self
    }
}

impl Default for FakeImageDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDecoder for FakeImageDecoder {
    fn probe(&self, src: &Path) -> Result<ImageInfo, ImageError> {
        self.images.get(src).cloned().unwrap_or_else(|| {
            Err(ImageError::Missing {
                path: src.to_path_buf(),
            })
        })
    }

    fn decode_first_frame(&self, src: &Path) -> Result<RasterImage, ImageError> {
        let info = self.probe(src)?;
        let pixel_count = info.width_px as usize * info.height_px as usize;

        Ok(RasterImage {
            width: info.width_px,
            height: info.height_px,
            rgba: vec![255; pixel_count * 4],
        })
    }
}

/// A file entry, for building `FakeDirectoryWalker` trees readably.
pub fn file(path: impl Into<PathBuf>) -> WalkEntry {
    WalkEntry {
        path: path.into(),
        is_dir: false,
    }
}

/// A directory entry the walk is allowed to descend into.
pub fn subdir(path: impl Into<PathBuf>) -> WalkEntry {
    WalkEntry {
        path: path.into(),
        is_dir: true,
    }
}

/// An in-memory directory tree.
///
/// A path is a directory exactly when it has been registered, whether with
/// contents or with a failure. Registering a path with `with_dir` while also
/// listing it as a `file(...)` entry of its parent is how a symlinked
/// directory is modelled: reachable, listable, but never descended into.
pub struct FakeDirectoryWalker {
    directories: HashMap<PathBuf, Result<Vec<WalkEntry>, WalkError>>,
}

impl FakeDirectoryWalker {
    pub fn new() -> Self {
        Self {
            directories: HashMap::new(),
        }
    }

    pub fn with_dir(mut self, dir: impl Into<PathBuf>, entries: Vec<WalkEntry>) -> Self {
        self.directories.insert(dir.into(), Ok(entries));
        self
    }

    pub fn with_failure(mut self, dir: impl Into<PathBuf>, err: WalkError) -> Self {
        self.directories.insert(dir.into(), Err(err));
        self
    }
}

impl Default for FakeDirectoryWalker {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryWalker for FakeDirectoryWalker {
    fn is_dir(&self, path: &Path) -> bool {
        self.directories.contains_key(path)
    }

    fn read_dir(&self, dir: &Path) -> Result<Vec<WalkEntry>, WalkError> {
        self.directories.get(dir).cloned().unwrap_or_else(|| {
            Err(WalkError::Missing {
                path: dir.to_path_buf(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::application::ports::ComposeEntry;
    use crate::domain::geometry::PageSize;

    use super::*;

    #[test]
    fn probe_returns_the_registered_document_info() {
        let info = DocumentInfo {
            page_count: 3,
            page_sizes: vec![PageSize::A4_PORTRAIT; 3],
            encrypted: false,
        };
        let engine = FakePdfEngine::new().with_document("/a.pdf", info.clone());
        assert_eq!(engine.probe(Path::new("/a.pdf")).unwrap(), info);
    }

    #[test]
    fn probe_returns_missing_for_unregistered_paths() {
        let engine = FakePdfEngine::new();
        let err = engine.probe(Path::new("/nope.pdf")).unwrap_err();
        assert!(matches!(err, PdfError::Missing { .. }));
    }

    #[test]
    fn registered_failures_are_returned_verbatim() {
        let engine = FakePdfEngine::new().with_failure(
            "/locked.pdf",
            PdfError::Encrypted {
                path: "/locked.pdf".into(),
            },
        );
        assert!(matches!(
            engine.probe(Path::new("/locked.pdf")).unwrap_err(),
            PdfError::Encrypted { .. }
        ));
    }

    #[test]
    fn compose_records_the_plan_it_received() {
        let engine = FakePdfEngine::new();
        let plan = ComposePlan {
            entries: vec![ComposeEntry::PdfPage {
                path: "/a.pdf".into(),
                page: PageIndex(0),
                rotation: Default::default(),
            }],
        };
        engine.compose(&plan, Path::new("/out.pdf")).unwrap();
        assert_eq!(engine.last_composed(), Some(plan));
    }

    #[test]
    fn rasterize_produces_an_image_matching_the_requested_width() {
        let engine = FakePdfEngine::new().with_document(
            "/a.pdf",
            DocumentInfo {
                page_count: 1,
                page_sizes: vec![PageSize::A4_PORTRAIT],
                encrypted: false,
            },
        );
        let image = engine
            .rasterize(
                Path::new("/a.pdf"),
                PageIndex(0),
                RasterSpec {
                    target_width_px: 200,
                },
            )
            .unwrap();
        assert_eq!(image.width, 200);
        assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
    }

    #[test]
    fn rasterize_records_the_request_it_received() {
        let engine = FakePdfEngine::new();

        // Unregistered on purpose: the request is recorded even when it fails.
        assert!(engine
            .rasterize(
                Path::new("/a.pdf"),
                PageIndex(3),
                RasterSpec {
                    target_width_px: 200,
                },
            )
            .is_err());

        assert_eq!(
            engine.last_rasterized(),
            Some((PathBuf::from("/a.pdf"), PageIndex(3), 200))
        );
    }

    #[test]
    fn image_probe_returns_the_registered_image_info() {
        let info = ImageInfo {
            width_px: 640,
            height_px: 480,
        };
        let decoder = FakeImageDecoder::new().with_image("/a.png", info.clone());
        assert_eq!(decoder.probe(Path::new("/a.png")).unwrap(), info);
    }

    #[test]
    fn image_probe_returns_registered_failures() {
        let decoder = FakeImageDecoder::new().with_failure(
            "/broken.png",
            ImageError::UnsupportedFormat {
                path: "/broken.png".into(),
            },
        );
        assert!(matches!(
            decoder.probe(Path::new("/broken.png")).unwrap_err(),
            ImageError::UnsupportedFormat { .. }
        ));
    }

    #[test]
    fn the_fake_walker_lists_a_registered_directory() {
        let walker = FakeDirectoryWalker::new()
            .with_dir("/scans", vec![file("/scans/a.pdf"), subdir("/scans/2024")]);

        assert!(walker.is_dir(Path::new("/scans")));
        assert_eq!(
            walker.read_dir(Path::new("/scans")).unwrap(),
            vec![file("/scans/a.pdf"), subdir("/scans/2024")]
        );
    }

    #[test]
    fn the_fake_walker_reports_missing_for_unregistered_directories() {
        let walker = FakeDirectoryWalker::new();

        assert!(!walker.is_dir(Path::new("/nope")));
        assert!(matches!(
            walker.read_dir(Path::new("/nope")).unwrap_err(),
            WalkError::Missing { .. }
        ));
    }

    #[test]
    fn the_fake_walker_returns_registered_failures_verbatim() {
        let walker = FakeDirectoryWalker::new().with_failure(
            "/locked",
            WalkError::Unreadable {
                path: "/locked".into(),
                reason: "permission denied".into(),
            },
        );

        // A registered failure still counts as a directory: the walk has to be
        // able to reach it before it can fail to list it.
        assert!(walker.is_dir(Path::new("/locked")));
        assert!(matches!(
            walker.read_dir(Path::new("/locked")).unwrap_err(),
            WalkError::Unreadable { .. }
        ));
    }
}
