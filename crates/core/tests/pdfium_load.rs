use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use std::path::PathBuf;

fn library_dir() -> PathBuf {
    // The fetch task places the shared library here relative to the repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pdfium")
}

#[test]
fn loads_the_bundled_pdfium_library() {
    let engine = PdfiumEngine::new(&library_dir()).expect("PDFium should load");
    assert!(!engine.version().is_empty());
}

#[test]
fn reports_a_clear_error_when_the_library_is_missing() {
    let err = PdfiumEngine::new(&PathBuf::from("/nonexistent")).unwrap_err();
    assert!(format!("{err}").contains("PDFium"));
}
