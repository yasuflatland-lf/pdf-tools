#![allow(dead_code)]

use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use pdfium_render::prelude::{
    PdfColor, PdfPageObjectsCommon, PdfPagePaperSize, PdfPagePaperStandardSize, PdfPagePathObject,
    PdfPageRenderRotation, PdfPoints, PdfRect, Pdfium, PdfiumError,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[path = "../../examples/support/exif_fixtures.rs"]
mod exif_fixtures;

pub fn library_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pdfium")
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn fixture(name: &str) -> PathBuf {
    ensure_generated();
    fixtures_dir().join(name)
}

pub fn image_fixture(name: &str) -> PathBuf {
    ensure_image_fixtures();
    fixtures_dir().join(name)
}

/// Returns the process-wide PDFium engine.
///
/// PDFium's bindings are a process-global singleton, so a second
/// `PdfiumEngine::new` in the same process fails with
/// `PdfiumLibraryBindingsAlreadyInitialized`. Handing out one shared engine
/// lets a test both generate fixtures and probe them.
pub fn engine() -> &'static PdfiumEngine {
    static ENGINE: OnceLock<PdfiumEngine> = OnceLock::new();

    ENGINE.get_or_init(|| {
        PdfiumEngine::new(&library_dir()).expect("PDFium should load for integration tests")
    })
}

pub fn ensure_generated() {
    ensure_image_fixtures();

    let directory = fixtures_dir();
    let multi_page_path = directory.join("multi_page.pdf");
    let mixed_size_path = directory.join("mixed_size.pdf");
    let rotated_page_path = directory.join("rotated_page.pdf");
    let corrupt_path = directory.join("corrupt.pdf");
    let needs_multi_page_bytes = !multi_page_path.exists() || !corrupt_path.exists();
    let needs_mixed_size_bytes = !mixed_size_path.exists();
    let needs_rotated_page_bytes = !rotated_page_path.exists();

    if !needs_multi_page_bytes && !needs_mixed_size_bytes && !needs_rotated_page_bytes {
        return;
    }

    let (multi_page_bytes, mixed_size_bytes, rotated_page_bytes) =
        engine().with_library(|pdfium| {
            let multi_page_bytes = needs_multi_page_bytes.then(|| {
                make_pdf(
                    pdfium,
                    &[
                        PdfPagePaperSize::a4(),
                        PdfPagePaperSize::a4(),
                        PdfPagePaperSize::a4(),
                    ],
                )
                .expect("multi-page fixture should be generated")
            });
            let mixed_size_bytes = needs_mixed_size_bytes.then(|| {
                make_pdf(
                    pdfium,
                    &[
                        PdfPagePaperSize::a4(),
                        PdfPagePaperSize::a4(),
                        PdfPagePaperSize::new_portrait(PdfPagePaperStandardSize::USLetterAnsiA),
                    ],
                )
                .expect("mixed-size fixture should be generated")
            });
            let rotated_page_bytes = needs_rotated_page_bytes.then(|| {
                make_rotated_pdf(pdfium).expect("rotated-page fixture should be generated")
            });

            (multi_page_bytes, mixed_size_bytes, rotated_page_bytes)
        });

    if let Some(bytes) = multi_page_bytes {
        write_atomic_if_absent(&multi_page_path, &bytes);

        if !corrupt_path.exists() {
            assert!(
                bytes.len() >= 100,
                "multi-page fixture should be at least 100 bytes"
            );
            let mut corrupt_bytes = bytes;
            corrupt_bytes[..100].fill(0);
            write_atomic_if_absent(&corrupt_path, &corrupt_bytes);
        }
    }

    if let Some(bytes) = mixed_size_bytes {
        write_atomic_if_absent(&mixed_size_path, &bytes);
    }

    if let Some(bytes) = rotated_page_bytes {
        write_atomic_if_absent(&rotated_page_path, &bytes);
    }
}

pub fn ensure_image_fixtures() {
    let directory = fixtures_dir();
    fs::create_dir_all(&directory).expect("fixture directory should be created");
    exif_fixtures::ensure_generated(&directory);
}

fn make_pdf(pdfium: &Pdfium, sizes: &[PdfPagePaperSize]) -> Result<Vec<u8>, PdfiumError> {
    let mut document = pdfium.create_new_pdf()?;

    for size in sizes {
        let mut page = document.pages_mut().create_page_at_end(*size)?;
        let rect = PdfRect::new(
            PdfPoints::new(page.height().value * 0.25),
            PdfPoints::new(page.width().value * 0.25),
            PdfPoints::new(page.height().value * 0.75),
            PdfPoints::new(page.width().value * 0.75),
        );
        let object =
            PdfPagePathObject::new_rect(&document, rect, None, None, Some(PdfColor::BLACK))?;
        page.objects_mut().add_path_object(object)?;
        page.regenerate_content()?;
        drop(page);
    }

    document.save_to_bytes()
}

fn make_rotated_pdf(pdfium: &Pdfium) -> Result<Vec<u8>, PdfiumError> {
    let mut document = pdfium.create_new_pdf()?;
    let mut page = document
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())?;
    page.set_rotation(PdfPageRenderRotation::Degrees90);
    drop(page);
    document.save_to_bytes()
}

fn write_atomic_if_absent(path: &Path, bytes: &[u8]) {
    if path.exists() {
        return;
    }

    let file_name = path
        .file_name()
        .expect("fixture path should have a file name")
        .to_string_lossy();
    let temporary_path = path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()));
    fs::write(&temporary_path, bytes).expect("temporary fixture should be written");

    if let Err(error) = fs::rename(&temporary_path, path) {
        if path.exists() {
            fs::remove_file(&temporary_path)
                .expect("redundant temporary fixture should be removed");
        } else {
            panic!("fixture should be renamed atomically: {error}");
        }
    }
}
