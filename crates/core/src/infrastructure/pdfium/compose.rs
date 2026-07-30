use super::probe::map_load_error;
use super::PdfiumEngine;
use crate::application::errors::PdfError;
use crate::application::ports::{ComposeEntry, ComposePlan, MergeReport};
use std::collections::HashMap;
use std::path::Path;

/// Builds a new PDF holding the plan's entries, in plan order.
///
/// Each distinct source path is opened once and kept for the duration of the
/// call, so a plan that draws many pages from one file -- or repeats the same
/// page -- never reopens it.
pub(super) fn compose(
    engine: &PdfiumEngine,
    plan: &ComposePlan,
    dest: &Path,
) -> Result<MergeReport, PdfError> {
    engine.with_library(|pdfium| {
        let mut destination = pdfium
            .create_new_pdf()
            .map_err(|error| write_error(dest, format!("failed to create document: {error}")))?;
        let mut sources = HashMap::new();
        let mut page_count = 0_u32;

        for entry in &plan.entries {
            let (path, page) = match entry {
                ComposeEntry::PdfPage { path, page } => (path, page),
                // Image entries are rejected rather than skipped so that a plan
                // is never silently composed with pages missing. Issue #25 adds
                // real image support.
                ComposeEntry::Image { path, .. } => {
                    return Err(PdfError::Unreadable {
                        path: path.clone(),
                        reason: "image composition is not supported yet".into(),
                    })
                }
            };

            if !path.is_file() {
                return Err(PdfError::Missing { path: path.clone() });
            }

            if !sources.contains_key(path) {
                let source = pdfium
                    .load_pdf_from_file(path, None)
                    .map_err(|error| map_load_error(path, error))?;
                sources.insert(path.clone(), source);
            }
            let source = sources
                .get(path)
                .expect("source was inserted immediately above");
            let source_page_count = source.pages().len() as u32;

            if page.0 >= source_page_count {
                return Err(PdfError::PageOutOfRange {
                    page: page.0,
                    count: source_page_count,
                });
            }

            destination
                .pages_mut()
                .copy_page_from_document(source, page.0 as i32, page_count as i32)
                .map_err(|error| PdfError::Unreadable {
                    path: path.clone(),
                    reason: format!("PDFium could not import page {}: {error}", page.0 + 1),
                })?;
            page_count += 1;
        }

        destination
            .save_to_file(dest)
            .map_err(|error| write_error(dest, error.to_string()))?;
        let bytes_written = std::fs::metadata(dest)
            .map_err(|error| write_error(dest, error.to_string()))?
            .len();

        Ok(MergeReport {
            page_count,
            bytes_written,
        })
    })
}

fn write_error(path: &Path, reason: String) -> PdfError {
    PdfError::WriteFailed {
        path: path.to_path_buf(),
        reason,
    }
}
