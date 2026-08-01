use crate::application::errors::{ImageError, PdfError};
use crate::application::ports::{ImageDecoder, PdfEngine};
use crate::domain::document::MergeDocument;
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::SlotId;
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

pub struct RasterizeSlot<'a> {
    pub pdf: &'a dyn PdfEngine,
    pub images: &'a dyn ImageDecoder,
}

impl RasterizeSlot<'_> {
    /// Renders one slot of the document to pixels.
    ///
    /// `target_width_px` is honoured for PDF pages. An image is decoded at its
    /// native size: `ImageDecoder` takes no target width, because an image
    /// source is a single page whose thumbnail the frontend scales anyway.
    pub fn execute(
        &self,
        document: &MergeDocument,
        slot_id: SlotId,
        target_width_px: u32,
    ) -> Result<RasterImage, RasterizeError> {
        let slot = document
            .plan()
            .slots()
            .iter()
            .find(|slot| slot.id == slot_id)
            .ok_or(RasterizeError::SlotNotFound(slot_id.0))?;
        let source = document.source_of(slot);

        match source.kind {
            SourceKind::Pdf => {
                Ok(self
                    .pdf
                    .rasterize(&source.path, slot.page, RasterSpec { target_width_px })?)
            }
            SourceKind::Image => Ok(self.images.decode_first_frame(&source.path)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::application::errors::{ImageError, PdfError};
    use crate::domain::document::MergeDocument;
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::{MergePlan, PageSlot};
    use crate::domain::source::{DocumentInfo, ImageInfo, SourceFile, SourceKind, SourceStatus};
    use crate::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    fn document(kind: SourceKind) -> MergeDocument {
        let path = match kind {
            SourceKind::Pdf => PathBuf::from("/document.pdf"),
            SourceKind::Image => PathBuf::from("/image.png"),
        };
        MergeDocument::new(
            MergePlan::new(vec![PageSlot {
                id: SlotId(7),
                source: SourceId(11),
                page: PageIndex(0),
                rotation: Default::default(),
            }]),
            vec![SourceFile {
                id: SourceId(11),
                path,
                kind,
                page_count: 1,
                page_sizes: match kind {
                    SourceKind::Pdf => vec![PageSize::A4_PORTRAIT],
                    SourceKind::Image => Vec::new(),
                },
                status: SourceStatus::Ready,
            }],
        )
    }

    #[test]
    fn a_pdf_slot_is_rasterized_at_the_requested_width() {
        let document = document(SourceKind::Pdf);
        let pdf = FakePdfEngine::new().with_document(
            "/document.pdf",
            DocumentInfo {
                page_count: 1,
                page_sizes: vec![PageSize::A4_PORTRAIT],
                encrypted: false,
            },
        );

        let image = RasterizeSlot {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&document, SlotId(7), 320)
        .unwrap();

        assert_eq!(image.width, 320);
    }

    #[test]
    fn an_image_slot_is_decoded_without_calling_the_pdf_engine() {
        let document = document(SourceKind::Image);
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
        .execute(&document, SlotId(7), 320)
        .unwrap();

        assert_eq!((image.width, image.height), (80, 60));
    }

    #[test]
    fn an_unknown_slot_is_reported_without_touching_either_engine() {
        let document = document(SourceKind::Pdf);

        let error = RasterizeSlot {
            pdf: &FakePdfEngine::new(),
            images: &FakeImageDecoder::new(),
        }
        .execute(&document, SlotId(99), 320)
        .unwrap_err();

        assert_eq!(error, RasterizeError::SlotNotFound(99));
    }

    #[test]
    fn a_pdf_engine_failure_carries_the_original_error() {
        let document = document(SourceKind::Pdf);
        let expected = PdfError::EngineUnavailable("test engine unavailable".into());
        let pdf = FakePdfEngine::new().with_failure("/document.pdf", expected.clone());

        let error = RasterizeSlot {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&document, SlotId(7), 320)
        .unwrap_err();

        assert_eq!(error, RasterizeError::Pdf(expected));
    }

    #[test]
    fn an_image_decoder_failure_surfaces_as_an_image_error() {
        let document = document(SourceKind::Image);
        let expected = ImageError::UnsupportedFormat {
            path: "/image.png".into(),
        };
        let images = FakeImageDecoder::new().with_failure("/image.png", expected.clone());

        let error = RasterizeSlot {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(&document, SlotId(7), 320)
        .unwrap_err();

        assert_eq!(error, RasterizeError::Image(expected));
    }
}
