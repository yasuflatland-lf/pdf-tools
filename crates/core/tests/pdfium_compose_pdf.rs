#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/phash.rs"]
mod phash;

use fixtures::{engine, fixture};
use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::{ComposeEntry, ComposePlan, PdfEngine};
use pdf_tools_core::domain::geometry::{PageSize, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use pdf_tools_core::domain::plan::Rotation;
use pdfium_render::prelude::{
    PdfColor, PdfPageObjectsCommon, PdfPagePaperSize, PdfPagePathObject, PdfPageRenderRotation,
    PdfPoints, PdfRect,
};
use phash::{average_hash, hamming_distance, render_page};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

fn temp_path(name: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "pdf-tools-compose-{}-{}-{name}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn page_rotation(path: &Path) -> PdfPageRenderRotation {
    engine().with_library(|pdfium| {
        let document = pdfium.load_pdf_from_file(path, None).unwrap();
        document.pages().get(0).unwrap().rotation().unwrap()
    })
}

/// A three-page PDF whose pages carry a black bar at a different height each.
///
/// The shared `multi_page.pdf` fixture draws the same mark on all three of its
/// A4 pages, so its pages hash identically and an ordering assertion made
/// against it would hold whichever page compose actually wrote. This fixture
/// exists so that the ordering assertion can fail.
fn distinct_pages_pdf() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();

    PATH.get_or_init(|| {
        let path = temp_path("distinct_pages.pdf");

        engine().with_library(|pdfium| {
            let mut document = pdfium.create_new_pdf().expect("document should be created");

            for band in 0..3 {
                let mut page = document
                    .pages_mut()
                    .create_page_at_end(PdfPagePaperSize::a4())
                    .expect("page should be created");
                let width = page.width().value;
                let height = page.height().value;
                let bottom = height * (0.05 + 0.3 * band as f32);
                let rect = PdfRect::new(
                    PdfPoints::new(bottom),
                    PdfPoints::new(width * 0.1),
                    PdfPoints::new(bottom + height * 0.25),
                    PdfPoints::new(width * 0.9),
                );
                let object =
                    PdfPagePathObject::new_rect(&document, rect, None, None, Some(PdfColor::BLACK))
                        .expect("bar should be created");
                page.objects_mut()
                    .add_path_object(object)
                    .expect("bar should be added to the page");
                page.regenerate_content()
                    .expect("page content should regenerate");
                drop(page);
            }

            document
                .save_to_file(&path)
                .expect("fixture should be saved");
        });

        path
    })
    .as_path()
}

#[test]
fn compose_writes_the_pages_in_plan_order() {
    let out = temp_path("out.pdf");
    let source = distinct_pages_pdf();
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: source.to_path_buf(),
                page: PageIndex(2),
                rotation: Default::default(),
            },
            ComposeEntry::PdfPage {
                path: source.to_path_buf(),
                page: PageIndex(0),
                rotation: Default::default(),
            },
        ],
    };
    let report = engine().compose(&plan, &out).unwrap();
    assert_eq!(report.page_count, 2);
    assert!(report.bytes_written > 0);

    // Verify by re-probing and comparing perceptual hashes -- never bytes.
    let info = engine().probe(&out).unwrap();
    assert_eq!(info.page_count(), 2);
    let spec = RasterSpec {
        target_width_px: 128,
    };
    let got_first = average_hash(&render_page(&out, PageIndex(0), spec));
    let got_second = average_hash(&render_page(&out, PageIndex(1), spec));
    let want_first = average_hash(&render_page(source, PageIndex(2), spec));
    let want_second = average_hash(&render_page(source, PageIndex(0), spec));

    // Guard the guard: the two source pages must hash far enough apart that the
    // assertions below can tell a swapped order from the requested one.
    assert!(
        hamming_distance(want_first, want_second) > 4,
        "the fixture pages are indistinguishable, so this test could not fail"
    );
    assert!(
        hamming_distance(got_first, want_first) <= 4,
        "page 0 is not the expected source page"
    );
    assert!(
        hamming_distance(got_second, want_second) <= 4,
        "page 1 is not the expected source page"
    );
}

#[test]
fn compose_preserves_each_page_size() {
    let out = temp_path("mixed.pdf");
    let source = fixture("mixed_size.pdf");
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: source.clone(),
                page: PageIndex(0),
                rotation: Default::default(),
            },
            ComposeEntry::PdfPage {
                path: source,
                page: PageIndex(2),
                rotation: Default::default(),
            },
        ],
    };
    engine().compose(&plan, &out).unwrap();
    let info = engine().probe(&out).unwrap();
    assert_ne!(
        info.page_sizes[0].size_class(),
        info.page_sizes[1].size_class()
    );
}

#[test]
fn the_same_page_may_be_included_twice() {
    let out = temp_path("dup.pdf");
    let source = fixture("multi_page.pdf");
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: source.clone(),
                page: PageIndex(0),
                rotation: Default::default(),
            },
            ComposeEntry::PdfPage {
                path: source,
                page: PageIndex(0),
                rotation: Default::default(),
            },
        ],
    };
    engine().compose(&plan, &out).unwrap();
    assert_eq!(engine().probe(&out).unwrap().page_count(), 2);
}

#[test]
fn a_slot_rotation_is_added_to_the_source_pages_rotation() {
    let out = temp_path("added-rotation.pdf");
    let source = fixture("rotated_page.pdf");
    assert_eq!(page_rotation(&source), PdfPageRenderRotation::Degrees90);
    let plan = ComposePlan {
        entries: vec![ComposeEntry::PdfPage {
            path: source,
            page: PageIndex(0),
            rotation: Rotation::from_quarter_turns(1),
        }],
    };

    engine().compose(&plan, &out).unwrap();

    assert_eq!(page_rotation(&out), PdfPageRenderRotation::Degrees180);
}

#[test]
fn compose_reports_missing_when_a_source_disappeared() {
    let plan = ComposePlan {
        entries: vec![ComposeEntry::PdfPage {
            path: "/gone.pdf".into(),
            page: PageIndex(0),
            rotation: Default::default(),
        }],
    };
    let err = engine()
        .compose(&plan, &temp_path("missing.pdf"))
        .unwrap_err();
    assert!(matches!(err, PdfError::Missing { .. }));
}

/// Pins that every source is checked before any entry is processed.
///
/// The first entry would fail on its own: its page index is past the end of the
/// three-page fixture. Reporting the *second* entry's absence instead is what
/// proves no page was copied and no image decoded before the plan was found to
/// name a file that has gone. The second entry is an image because that is the
/// expensive case — an image is decoded rather than copied.
#[test]
fn compose_reports_a_missing_source_before_processing_any_entry() {
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: fixture("multi_page.pdf"),
                page: PageIndex(99),
                rotation: Default::default(),
            },
            ComposeEntry::Image {
                path: "/gone.png".into(),
                fit_to: PageSize::A4_PORTRAIT,
                rotation: Default::default(),
            },
        ],
    };

    let err = engine()
        .compose(&plan, &temp_path("missing-checked-first.pdf"))
        .unwrap_err();

    assert_eq!(
        err,
        PdfError::Missing {
            path: "/gone.png".into()
        }
    );
}

#[test]
fn compose_reports_write_failure_for_an_unwritable_destination() {
    let plan = ComposePlan {
        entries: vec![ComposeEntry::PdfPage {
            path: fixture("multi_page.pdf"),
            page: PageIndex(0),
            rotation: Default::default(),
        }],
    };
    let err = engine()
        .compose(&plan, Path::new("/nonexistent-dir/out.pdf"))
        .unwrap_err();
    assert!(matches!(err, PdfError::WriteFailed { .. }));
}
