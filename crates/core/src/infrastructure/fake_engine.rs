use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::application::errors::{ImageError, PdfError};
use crate::application::ports::{ComposePlan, ImageDecoder, MergeReport, PdfEngine};
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use crate::domain::source::{DocumentInfo, ImageInfo};

pub struct FakePdfEngine {
    documents: HashMap<PathBuf, Result<DocumentInfo, PdfError>>,
    last_composed: Mutex<Option<ComposePlan>>,
}

impl FakePdfEngine {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            last_composed: Mutex::new(None),
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
}
