#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{engine, fixture};
use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::PdfEngine;
use pdf_tools_core::domain::geometry::PageSize;
use std::path::Path;

#[test]
fn probe_reports_page_count_and_sizes() {
    let engine = engine();
    let info = engine.probe(&fixture("multi_page.pdf")).unwrap();
    assert_eq!(info.page_count(), 3);
    assert_eq!(info.page_sizes.len(), 3);
    assert_eq!(
        info.page_sizes[0].size_class(),
        PageSize::A4_PORTRAIT.size_class()
    );
    assert!(!info.encrypted);
}

#[test]
fn probe_reports_per_page_sizes_for_mixed_documents() {
    let info = engine().probe(&fixture("mixed_size.pdf")).unwrap();
    assert_eq!(info.page_sizes.len(), 3);
    assert_ne!(
        info.page_sizes[0].size_class(),
        info.page_sizes[2].size_class()
    );
}

#[test]
fn probe_detects_encrypted_documents() {
    let err = engine().probe(&fixture("encrypted.pdf")).unwrap_err();
    assert!(matches!(err, PdfError::Encrypted { .. }));
}

#[test]
fn probe_reports_unreadable_for_corrupt_documents() {
    let err = engine().probe(&fixture("corrupt.pdf")).unwrap_err();
    assert!(matches!(err, PdfError::Unreadable { .. }));
}

#[test]
fn probe_reports_missing_for_absent_files() {
    let err = engine()
        .probe(Path::new("/definitely/not/here.pdf"))
        .unwrap_err();
    assert!(matches!(err, PdfError::Missing { .. }));
}
