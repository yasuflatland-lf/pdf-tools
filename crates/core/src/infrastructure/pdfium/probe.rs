use super::PdfiumEngine;
use crate::application::errors::PdfError;
use crate::application::ports::{ComposePlan, MergeReport, PdfEngine};
use crate::domain::geometry::{PageSize, RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use crate::domain::source::DocumentInfo;
use pdfium_render::prelude::{PdfiumError, PdfiumInternalError};
use std::io::ErrorKind;
use std::path::Path;

impl PdfEngine for PdfiumEngine {
    fn probe(&self, src: &Path) -> Result<DocumentInfo, PdfError> {
        if !src.is_file() {
            return Err(PdfError::Missing {
                path: src.to_path_buf(),
            });
        }

        self.with_library(|pdfium| {
            let document = pdfium
                .load_pdf_from_file(src, None)
                .map_err(|error| map_load_error(src, error))?;
            let pages = document.pages();
            let page_count = pages.len() as u32;
            let mut page_sizes = Vec::with_capacity(page_count as usize);

            for index in pages.as_range() {
                let page = pages.get(index).map_err(|error| PdfError::Unreadable {
                    path: src.to_path_buf(),
                    reason: format!("failed to inspect page {}: {error}", index + 1),
                })?;
                let page_size =
                    PageSize::new(page.width().value, page.height().value).ok_or_else(|| {
                        PdfError::Unreadable {
                            path: src.to_path_buf(),
                            reason: format!(
                                "page {} has a non-positive or non-finite size",
                                index + 1
                            ),
                        }
                    })?;
                page_sizes.push(page_size);
            }

            Ok(DocumentInfo {
                page_count,
                page_sizes,
                encrypted: false,
            })
        })
    }

    fn rasterize(
        &self,
        src: &Path,
        page: PageIndex,
        spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        super::rasterize::rasterize_page(self, src, page, spec)
    }

    /// Composition is implemented in task #24.
    fn compose(&self, plan: &ComposePlan, dest: &Path) -> Result<MergeReport, PdfError> {
        super::compose::compose(self, plan, dest)
    }
}

pub(super) fn map_load_error(src: &Path, error: PdfiumError) -> PdfError {
    match error {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
            PdfError::Encrypted {
                path: src.to_path_buf(),
            }
        }
        PdfiumError::IoError(error) if error.kind() == ErrorKind::NotFound => PdfError::Missing {
            path: src.to_path_buf(),
        },
        error => PdfError::Unreadable {
            path: src.to_path_buf(),
            reason: format!("PDFium could not open the document: {error}"),
        },
    }
}
