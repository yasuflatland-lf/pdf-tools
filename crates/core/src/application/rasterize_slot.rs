use std::path::PathBuf;

use crate::application::errors::{ImageError, PdfError};
use crate::application::ports::{ImageDecoder, PdfEngine};
use crate::domain::document::MergeDocument;
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::{PageIndex, SlotId};
use crate::domain::source::SourceKind;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RasterizeError {
    #[error("slot {0} is not in the plan")]
    SlotNotFound(u64),
    #[error(transparent)]
    Pdf(#[from] PdfError),
    #[error(transparent)]
    Image(#[from] ImageError),
}

/// Everything rendering one slot needs, lifted out of the document.
///
/// Resolving is a separate step from rendering so that a caller holding the
/// session lock can copy one path and let go, instead of either holding the
/// lock across an engine call or copying the whole document. A thumbnail is
/// fetched per visible card, so that copy would scale with the plan on every
/// scroll step; this one does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotTarget {
    pub path: PathBuf,
    pub kind: SourceKind,
    pub page: PageIndex,
}

impl SlotTarget {
    /// Resolves a slot of the document to the source it names.
    pub fn resolve(document: &MergeDocument, slot_id: SlotId) -> Result<Self, RasterizeError> {
        let slot = document
            .plan()
            .slots()
            .iter()
            .find(|slot| slot.id == slot_id)
            .ok_or(RasterizeError::SlotNotFound(slot_id.0))?;
        let source = document.source_of(slot);

        Ok(Self {
            path: source.path.clone(),
            kind: source.kind,
            page: slot.page,
        })
    }
}

pub struct RasterizeSlot<'a> {
    pub pdf: &'a dyn PdfEngine,
    pub images: &'a dyn ImageDecoder,
}

impl RasterizeSlot<'_> {
    /// Renders one resolved slot to pixels.
    ///
    /// `target_width_px` is honoured for PDF pages. An image is decoded at its
    /// native size: `ImageDecoder` takes no target width, because an image
    /// source is a single page whose thumbnail the frontend scales anyway.
    pub fn execute(
        &self,
        target: &SlotTarget,
        target_width_px: u32,
    ) -> Result<RasterImage, RasterizeError> {
        match target.kind {
            SourceKind::Pdf => {
                Ok(self
                    .pdf
                    .rasterize(&target.path, target.page, RasterSpec { target_width_px })?)
            }
            SourceKind::Image => Ok(self.images.decode_first_frame(&target.path)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::application::errors::{ImageError, PdfError};
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::SourceId;
    use crate::domain::plan::{MergePlan, PageSlot};
    use crate::domain::source::{DocumentInfo, ImageInfo, SourceFile, SourceStatus};
    use crate::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    /// Three sources and four slots, so a test can name a slot that is neither
    /// the first in the plan, nor page zero, nor from the first source.
    fn multi_source_document() -> MergeDocument {
        MergeDocument::new(
            MergePlan::new(vec![
                slot(1, 20, 0),
                slot(2, 22, 2),
                slot(3, 21, 0),
                slot(4, 20, 1),
            ]),
            vec![
                pdf_source(20, "/first.pdf", 2),
                image_source(21, "/photo.png"),
                pdf_source(22, "/second.pdf", 3),
            ],
        )
    }

    fn slot(id: u64, source: u64, page: u32) -> PageSlot {
        PageSlot {
            id: SlotId(id),
            source: SourceId(source),
            page: PageIndex(page),
            rotation: Default::default(),
        }
    }

    fn pdf_source(id: u64, path: &str, page_count: u32) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path: PathBuf::from(path),
            kind: SourceKind::Pdf,
            page_count,
            page_sizes: vec![PageSize::A4_PORTRAIT; page_count as usize],
            status: SourceStatus::Ready,
        }
    }

    fn image_source(id: u64, path: &str) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path: PathBuf::from(path),
            kind: SourceKind::Image,
            page_count: 1,
            page_sizes: Vec::new(),
            status: SourceStatus::Ready,
        }
    }

    fn pdf_info(page_count: u32) -> DocumentInfo {
        DocumentInfo {
            page_count,
            page_sizes: vec![PageSize::A4_PORTRAIT; page_count as usize],
            encrypted: false,
        }
    }

    fn pdf_target(path: &str, page: u32) -> SlotTarget {
        SlotTarget {
            path: PathBuf::from(path),
            kind: SourceKind::Pdf,
            page: PageIndex(page),
        }
    }

    fn image_target(path: &str) -> SlotTarget {
        SlotTarget {
            path: PathBuf::from(path),
            kind: SourceKind::Image,
            page: PageIndex(0),
        }
    }

    #[test]
    fn a_slot_resolves_to_the_path_kind_and_page_it_names() {
        let document = multi_source_document();

        assert_eq!(
            SlotTarget::resolve(&document, SlotId(2)).unwrap(),
            pdf_target("/second.pdf", 2)
        );
    }

    #[test]
    fn an_unknown_slot_is_not_resolved() {
        let document = multi_source_document();

        let error = SlotTarget::resolve(&document, SlotId(99)).unwrap_err();

        assert_eq!(error, RasterizeError::SlotNotFound(99));
    }

    #[test]
    fn the_engine_is_asked_for_the_named_slots_own_path_and_page() {
        let document = multi_source_document();
        let pdf = FakePdfEngine::new()
            .with_document("/first.pdf", pdf_info(2))
            .with_document("/second.pdf", pdf_info(3));

        RasterizeSlot {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&SlotTarget::resolve(&document, SlotId(2)).unwrap(), 320)
        .unwrap();

        assert_eq!(
            pdf.last_rasterized(),
            Some((PathBuf::from("/second.pdf"), PageIndex(2), 320))
        );
    }

    #[test]
    fn an_image_slot_is_decoded_from_its_own_source_among_several() {
        let document = multi_source_document();
        let images = FakeImageDecoder::new().with_image(
            "/photo.png",
            ImageInfo {
                width_px: 48,
                height_px: 24,
            },
        );

        let image = RasterizeSlot {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(&SlotTarget::resolve(&document, SlotId(3)).unwrap(), 320)
        .unwrap();

        assert_eq!((image.width, image.height), (48, 24));
    }

    #[test]
    fn a_pdf_slot_is_rasterized_at_the_requested_width() {
        let pdf = FakePdfEngine::new().with_document("/document.pdf", pdf_info(1));

        let image = RasterizeSlot {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&pdf_target("/document.pdf", 0), 320)
        .unwrap();

        assert_eq!(image.width, 320);
    }

    #[test]
    fn an_image_slot_is_decoded_without_calling_the_pdf_engine() {
        let images = FakeImageDecoder::new().with_image(
            "/image.png",
            ImageInfo {
                width_px: 80,
                height_px: 60,
            },
        );

        let image = RasterizeSlot {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(&image_target("/image.png"), 320)
        .unwrap();

        assert_eq!((image.width, image.height), (80, 60));
    }

    #[test]
    fn a_pdf_engine_failure_carries_the_original_error() {
        let expected = PdfError::EngineUnavailable("test engine unavailable".into());
        let pdf = FakePdfEngine::new().with_failure("/document.pdf", expected.clone());

        let error = RasterizeSlot {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&pdf_target("/document.pdf", 0), 320)
        .unwrap_err();

        assert_eq!(error, RasterizeError::Pdf(expected));
    }

    #[test]
    fn an_image_decoder_failure_surfaces_as_an_image_error() {
        let expected = ImageError::UnsupportedFormat {
            path: "/image.png".into(),
        };
        let images = FakeImageDecoder::new().with_failure("/image.png", expected.clone());

        let error = RasterizeSlot {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(&image_target("/image.png"), 320)
        .unwrap_err();

        assert_eq!(error, RasterizeError::Image(expected));
    }
}
