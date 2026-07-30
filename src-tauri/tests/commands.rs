use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use pdf_tools_core::application::compose::ProgressSink;
use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::{ComposePlan, MergeReport, PdfEngine};
use pdf_tools_core::domain::geometry::{PageSize, RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use pdf_tools_core::domain::source::{DocumentInfo, ImageInfo};
use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};
use pdf_tools_lib::presentation::commands::{
    add_sources_inner, compose_inner, insert_at_inner, rasterize_slot_inner, redo_inner,
    remove_slots_inner, reorder_inner, undo_inner,
};
use pdf_tools_lib::presentation::dto::PlanSnapshot;
use pdf_tools_lib::presentation::state::AppState;
use tempfile::TempDir;

const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
static TEMP_DIR: OnceLock<TempDir> = OnceLock::new();
static SOURCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Wraps the fake engine so a merge actually writes its destination, which lets
/// these tests assert on the output file.
struct WritingPdfEngine {
    inner: FakePdfEngine,
}

impl PdfEngine for WritingPdfEngine {
    fn probe(&self, src: &Path) -> Result<DocumentInfo, PdfError> {
        self.inner.probe(src)
    }

    fn rasterize(
        &self,
        src: &Path,
        page: PageIndex,
        spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        self.inner.rasterize(src, page, spec)
    }

    fn compose(&self, plan: &ComposePlan, dest: &Path) -> Result<MergeReport, PdfError> {
        let report = self.inner.compose(plan, dest)?;
        let bytes = b"%PDF-1.7\n%%EOF\n";
        std::fs::write(dest, bytes).map_err(|error| PdfError::WriteFailed {
            path: dest.to_path_buf(),
            reason: error.to_string(),
        })?;
        Ok(MergeReport {
            page_count: report.page_count,
            bytes_written: bytes.len() as u64,
        })
    }
}

struct NullProgress;

impl ProgressSink for NullProgress {
    fn report(&self, _done: u32, _total: u32) {}
}

#[derive(Default)]
struct RecordingProgress {
    reports: Mutex<Vec<(u32, u32)>>,
}

impl ProgressSink for RecordingProgress {
    fn report(&self, done: u32, total: u32) {
        self.reports
            .lock()
            .expect("progress reports lock should not be poisoned")
            .push((done, total));
    }
}

struct PausingPdfEngine {
    inner: WritingPdfEngine,
    started: mpsc::Sender<()>,
    resume: Mutex<mpsc::Receiver<()>>,
}

impl PdfEngine for PausingPdfEngine {
    fn probe(&self, src: &Path) -> Result<DocumentInfo, PdfError> {
        self.inner.probe(src)
    }

    fn rasterize(
        &self,
        src: &Path,
        page: PageIndex,
        spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        self.inner.rasterize(src, page, spec)
    }

    fn compose(&self, plan: &ComposePlan, dest: &Path) -> Result<MergeReport, PdfError> {
        self.started
            .send(())
            .expect("the test should be waiting for the merge to start");
        self.resume
            .lock()
            .expect("resume lock should not be poisoned")
            .recv()
            .expect("the test should release the merge");
        self.inner.compose(plan, dest)
    }
}

fn temp_path(name: &str) -> PathBuf {
    TEMP_DIR
        .get_or_init(|| tempfile::tempdir().expect("temporary directory should be created"))
        .path()
        .join(name)
}

fn unique_source_path() -> PathBuf {
    let sequence = SOURCE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    temp_path(&format!("document-{sequence}.pdf"))
}

fn fake_pdf() -> FakePdfEngine {
    let document = DocumentInfo {
        page_count: 3,
        page_sizes: vec![PageSize::A4_PORTRAIT; 3],
        encrypted: false,
    };

    FakePdfEngine::new()
        .with_document("/a.pdf", document.clone())
        .with_document("/deep/dir/invoice.pdf", document)
}

fn fake_images() -> FakeImageDecoder {
    FakeImageDecoder::new()
}

fn pdf_info() -> DocumentInfo {
    DocumentInfo {
        page_count: 1,
        page_sizes: vec![PageSize::A4_PORTRAIT],
        encrypted: false,
    }
}

fn add_sources(state: &AppState, paths: &[PathBuf]) -> u64 {
    add_sources_inner(
        state,
        paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    )
    .unwrap()
    .slots
    .last()
    .expect("the source should contribute a slot")
    .id
}

fn state_with_pdf(pages: u32) -> AppState {
    let source = unique_source_path();
    std::fs::write(&source, b"fixture").expect("source fixture should be created");
    let state = AppState::with_engines(
        Arc::new(WritingPdfEngine {
            inner: FakePdfEngine::new().with_document(
                &source,
                DocumentInfo {
                    page_count: pages,
                    page_sizes: vec![PageSize::A4_PORTRAIT; pages as usize],
                    encrypted: false,
                },
            ),
        }),
        Arc::new(FakeImageDecoder::new()),
    );
    add_sources_inner(&state, vec![source.to_string_lossy().into_owned()]).unwrap();
    state
}

fn state_with_missing_source() -> AppState {
    let state = state_with_pdf(1);
    let source = state
        .session()
        .sources()
        .first()
        .expect("the state should contain a source")
        .path
        .clone();
    std::fs::remove_file(source).expect("source fixture should be removed");
    state
}

fn snapshot_of(state: &AppState) -> PlanSnapshot {
    PlanSnapshot::from_session(&state.session())
}

fn state_with_pdf_and_image() -> (AppState, u64) {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document(
            "/document.pdf",
            DocumentInfo {
                page_count: 3,
                page_sizes: vec![PageSize::A4_PORTRAIT; 3],
                encrypted: false,
            },
        )),
        Arc::new(FakeImageDecoder::new().with_image(
            "/image.png",
            ImageInfo {
                width_px: 40,
                height_px: 30,
            },
        )),
    );
    add_sources_inner(&state, vec!["/document.pdf".into()]).unwrap();
    let snapshot = add_sources_inner(&state, vec!["/image.png".into()]).unwrap();
    let image_slot_id = snapshot.slots.last().expect("image slot").id;
    (state, image_slot_id)
}

fn png_width(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(
        bytes[16..20]
            .try_into()
            .expect("a PNG should contain an IHDR width"),
    )
}

#[test]
fn reorder_returns_a_snapshot_in_the_new_order() {
    let state = state_with_pdf(3);
    let snapshot = reorder_inner(&state, 0, 1, 2).unwrap();
    assert_eq!(snapshot.slots[2].page, 0);
}

#[test]
fn undo_flags_reflect_the_history() {
    let state = state_with_pdf(3);
    let snapshot = remove_slots_inner(&state, vec![0]).unwrap();
    assert!(snapshot.can_undo);
    assert!(!snapshot.can_redo);
    let snapshot = undo_inner(&state).unwrap();
    assert!(snapshot.can_redo);
}

#[test]
fn redo_after_undo_restores_the_change() {
    let state = state_with_pdf(3);
    let removed = remove_slots_inner(&state, vec![0]).unwrap();
    undo_inner(&state).unwrap();

    let redone = redo_inner(&state).unwrap();

    assert_eq!(redone.slots, removed.slots);
    assert!(redone.can_undo);
    assert!(!redone.can_redo);
}

#[test]
fn undo_on_a_fresh_state_is_a_no_op() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new()),
    );

    let snapshot = undo_inner(&state).unwrap();

    assert!(snapshot.slots.is_empty());
    assert!(!snapshot.can_undo);
    assert!(!snapshot.can_redo);
}

#[test]
fn inserting_inside_a_group_marks_the_source_ungrouped_in_the_snapshot() {
    let (state, image_slot_id) = state_with_pdf_and_image();
    let snapshot = insert_at_inner(&state, 1, vec![image_slot_id]).unwrap();
    let pdf_source = snapshot
        .sources
        .iter()
        .find(|source| source.kind == "pdf")
        .unwrap();
    assert_eq!(pdf_source.grouping, "ungrouped");
}

#[test]
fn add_sources_returns_a_snapshot_containing_the_new_slots() {
    let state = AppState::with_engines(Arc::new(fake_pdf()), Arc::new(fake_images()));
    let snapshot = add_sources_inner(&state, vec!["/a.pdf".into()]).unwrap();

    assert_eq!(snapshot.slots.len(), 3);
    assert_eq!(snapshot.sources[0].kind, "pdf");
}

#[test]
fn the_snapshot_exposes_a_display_file_name() {
    let state = AppState::with_engines(Arc::new(fake_pdf()), Arc::new(fake_images()));
    let snapshot = add_sources_inner(&state, vec!["/deep/dir/invoice.pdf".into()]).unwrap();

    assert_eq!(snapshot.sources[0].file_name, "invoice.pdf");
}

#[test]
fn rasterize_slot_returns_png_bytes_for_a_known_slot() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document("/known.pdf", pdf_info())),
        Arc::new(FakeImageDecoder::new()),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.pdf")]);

    let bytes = rasterize_slot_inner(&state, slot_id, 200).unwrap();

    assert_eq!(&bytes[..PNG_MAGIC.len()], PNG_MAGIC);
    assert_eq!(png_width(&bytes), 200);
}

#[test]
fn rasterize_slot_reports_an_error_for_an_unknown_slot() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new()),
    );

    assert!(rasterize_slot_inner(&state, 9999, 200).is_err());
}

#[test]
fn rasterize_slot_returns_png_bytes_for_an_image_source() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new().with_image(
            "/known.png",
            ImageInfo {
                width_px: 40,
                height_px: 30,
            },
        )),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.png")]);

    let bytes = rasterize_slot_inner(&state, slot_id, 200).unwrap();

    assert_eq!(&bytes[..PNG_MAGIC.len()], PNG_MAGIC);
    assert_eq!(png_width(&bytes), 40);
}

#[test]
fn rasterize_slot_surfaces_a_rasterize_failure() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document(
            "/known.pdf",
            DocumentInfo {
                page_count: 1,
                page_sizes: vec![],
                encrypted: false,
            },
        )),
        Arc::new(FakeImageDecoder::new()),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.pdf")]);

    let error = rasterize_slot_inner(&state, slot_id, 200).unwrap_err();

    assert_eq!(error, "page 0 is out of range (the document has 1 pages)");
}

#[test]
fn compose_writes_to_the_requested_destination() {
    let state = state_with_pdf(2);
    let dest = temp_path("merged.pdf");
    let report = compose_inner(&state, &dest, &NullProgress).unwrap();
    assert_eq!(report.page_count, 2);
    assert!(dest.exists());
}

#[test]
fn compose_surfaces_a_readable_message_when_a_source_vanished() {
    let state = state_with_missing_source();
    let err = compose_inner(&state, &temp_path("x.pdf"), &NullProgress).unwrap_err();
    assert!(
        err.contains("no longer present"),
        "unhelpful message: {err}"
    );
}

#[test]
fn compose_does_not_alter_the_plan() {
    let state = state_with_pdf(2);
    let before = snapshot_of(&state);
    compose_inner(&state, &temp_path("y.pdf"), &NullProgress).unwrap();
    assert_eq!(snapshot_of(&state), before);
}

#[test]
fn compose_reports_progress_for_every_page() {
    let state = state_with_pdf(2);
    let progress = RecordingProgress::default();

    compose_inner(&state, &temp_path("progress.pdf"), &progress).unwrap();

    let reports = progress
        .reports
        .lock()
        .expect("progress reports lock should not be poisoned");
    assert_eq!(reports.last(), Some(&(2, 2)));
}

#[test]
fn the_session_stays_readable_while_a_merge_runs() {
    let source = unique_source_path();
    std::fs::write(&source, b"fixture").expect("source fixture should be created");
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let engine = PausingPdfEngine {
        inner: WritingPdfEngine {
            inner: FakePdfEngine::new().with_document(
                &source,
                DocumentInfo {
                    page_count: 2,
                    page_sizes: vec![PageSize::A4_PORTRAIT; 2],
                    encrypted: false,
                },
            ),
        },
        started: started_tx,
        resume: Mutex::new(resume_rx),
    };
    let state = Arc::new(AppState::with_engines(
        Arc::new(engine),
        Arc::new(FakeImageDecoder::new()),
    ));
    add_sources_inner(&state, vec![source.to_string_lossy().into_owned()]).unwrap();

    let worker_state = Arc::clone(&state);
    let dest = temp_path("non-blocking.pdf");
    let worker = std::thread::spawn(move || compose_inner(&worker_state, &dest, &NullProgress));
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("the merge should start");

    let reader_state = Arc::clone(&state);
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        snapshot_tx
            .send(snapshot_of(&reader_state))
            .expect("the test should receive the snapshot");
    });
    let snapshot = snapshot_rx.recv_timeout(Duration::from_secs(5));

    resume_tx.send(()).expect("the merge should be released");
    let report = worker
        .join()
        .expect("merge thread should not panic")
        .unwrap();
    reader.join().expect("snapshot thread should not panic");

    assert_eq!(
        snapshot
            .expect("the session lock should remain readable")
            .slots
            .len(),
        2
    );
    assert_eq!(report.page_count, 2);
}
