use std::path::{Path, PathBuf};

use crate::application::errors::PdfError;
use crate::application::ports::{ComposeEntry, ComposePlan, MergeReport, PdfEngine};
use crate::domain::document::MergeDocument;
use crate::domain::source::{SourceKind, SourceStatus};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ComposeError {
    #[error("the source file is no longer present: {path}")]
    SourceMissing { path: PathBuf },
    #[error("nothing to merge: the plan is empty")]
    EmptyPlan,
    #[error("nothing to merge: the document has no usable pages")]
    NoUsableSources,
    #[error(transparent)]
    Engine(#[from] PdfError),
}

pub trait ProgressSink: Send + Sync {
    fn report(&self, done: u32, total: u32);
}

pub struct Compose<'a> {
    pub pdf: &'a dyn PdfEngine,
}

impl Compose<'_> {
    /// Resolves the plan into a `ComposePlan` (turning `SourceId`s into paths and
    /// assigning every image the plan's dominant page size), verifies that every
    /// source still exists, then hands the work to the engine.
    ///
    /// Slots whose source is in a failed state contribute no entry. Every
    /// remaining source is checked before the engine runs, so a missing file
    /// can never leave a half-written output behind.
    pub fn execute(
        &self,
        document: &MergeDocument,
        dest: &Path,
        progress: &dyn ProgressSink,
    ) -> Result<MergeReport, ComposeError> {
        if document.plan().is_empty() {
            return Err(ComposeError::EmptyPlan);
        }

        let fit_to = document.dominant_page_size();
        let entries = document
            .plan()
            .slots()
            .iter()
            .filter(|slot| document.source_of(slot).status() == SourceStatus::Ready)
            .map(|slot| {
                let source = document.source_of(slot);
                match source.kind() {
                    SourceKind::Pdf => ComposeEntry::PdfPage {
                        path: source.path().to_path_buf(),
                        page: slot.page,
                        rotation: slot.rotation,
                    },
                    SourceKind::Image => ComposeEntry::Image {
                        path: source.path().to_path_buf(),
                        fit_to,
                        rotation: slot.rotation,
                    },
                }
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return Err(ComposeError::NoUsableSources);
        }

        for entry in &entries {
            let path = match entry {
                ComposeEntry::PdfPage { path, .. } | ComposeEntry::Image { path, .. } => path,
            };
            if !path.is_file() {
                return Err(ComposeError::SourceMissing { path: path.clone() });
            }
        }

        let total = entries.len() as u32;
        for done in 1..=total {
            progress.report(done, total);
        }

        self.pdf
            .compose(&ComposePlan { entries }, dest)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use tempfile::{tempdir, TempDir};

    use super::*;
    use crate::application::ports::ComposeEntry;
    use crate::domain::document::MergeDocument;
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::{MergePlan, PageSlot, Rotation};
    use crate::domain::source::{SourceFile, SourceKind, SourceStatus};
    use crate::infrastructure::fake_engine::FakePdfEngine;

    fn letter() -> PageSize {
        PageSize::new(612.0, 792.0).expect("Letter page size should be valid")
    }

    struct NullProgress;

    impl ProgressSink for NullProgress {
        fn report(&self, _done: u32, _total: u32) {}
    }

    #[derive(Default)]
    struct RecordingProgress {
        reports: Mutex<Vec<(u32, u32)>>,
    }

    impl RecordingProgress {
        fn reports(&self) -> Vec<(u32, u32)> {
            self.reports
                .lock()
                .expect("recording progress mutex should not be poisoned")
                .clone()
        }
    }

    impl ProgressSink for RecordingProgress {
        fn report(&self, done: u32, total: u32) {
            self.reports
                .lock()
                .expect("recording progress mutex should not be poisoned")
                .push((done, total));
        }
    }

    fn create_file(temp_dir: &TempDir, name: &str) -> PathBuf {
        let path = temp_dir.path().join(name);
        File::create(&path).expect("fixture file should be created");
        path
    }

    fn slot(id: u64, source: u64, page: u32) -> PageSlot {
        PageSlot {
            id: SlotId(id),
            source: SourceId(source),
            page: PageIndex(page),
            rotation: Default::default(),
        }
    }

    fn pdf_source(id: u64, path: PathBuf, page_sizes: Vec<PageSize>) -> SourceFile {
        SourceFile::ready_pdf(SourceId(id), path, page_sizes)
    }

    fn image_source(id: u64, path: PathBuf) -> SourceFile {
        SourceFile::ready_image(SourceId(id), path)
    }

    fn plan_with_pdf_and_image() -> (TempDir, MergeDocument) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "document.pdf");
        let image_path = create_file(&temp_dir, "image.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, pdf_path, vec![PageSize::A4_PORTRAIT]),
            image_source(20, image_path),
        ];
        (temp_dir, MergeDocument::new(plan, sources))
    }

    fn plan_with_letter_pdf_and_image() -> (TempDir, MergeDocument) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "letter.pdf");
        let image_path = create_file(&temp_dir, "image.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, pdf_path, vec![letter()]),
            image_source(20, image_path),
        ];
        (temp_dir, MergeDocument::new(plan, sources))
    }

    fn plan_with_nonexistent_source() -> (TempDir, MergeDocument) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let existing_path = create_file(&temp_dir, "existing.pdf");
        let missing_path = temp_dir.path().join("missing.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, existing_path, vec![PageSize::A4_PORTRAIT]),
            image_source(20, missing_path),
        ];
        (temp_dir, MergeDocument::new(plan, sources))
    }

    fn plan_with_three_slots() -> (TempDir, MergeDocument) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "two-pages.pdf");
        let image_path = create_file(&temp_dir, "image.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0), slot(3, 10, 1)]);
        let sources = vec![
            pdf_source(
                10,
                pdf_path,
                vec![PageSize::A4_PORTRAIT, PageSize::A4_PORTRAIT],
            ),
            image_source(20, image_path),
        ];
        (temp_dir, MergeDocument::new(plan, sources))
    }

    #[test]
    fn the_resolved_plan_follows_the_slot_order() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, document) = plan_with_pdf_and_image();
        Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap();
        let composed = engine.last_composed().unwrap();
        assert!(matches!(composed.entries[0], ComposeEntry::PdfPage { .. }));
        assert!(matches!(composed.entries[1], ComposeEntry::Image { .. }));
    }

    #[test]
    fn every_image_is_fitted_to_the_dominant_page_size() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, document) = plan_with_letter_pdf_and_image();
        Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap();
        let composed = engine.last_composed().unwrap();
        match &composed.entries[1] {
            ComposeEntry::Image { fit_to, .. } => {
                assert_eq!(fit_to.size_class(), letter().size_class());
            }
            other => panic!("expected an image entry, got {other:?}"),
        }
    }

    #[test]
    fn rotated_slots_produce_entries_carrying_their_rotation() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "two-pages.pdf");
        let mut first = slot(1, 10, 0);
        first.rotation = Rotation::from_quarter_turns(1);
        let mut second = slot(2, 10, 1);
        second.rotation = Rotation::from_quarter_turns(3);
        let document = MergeDocument::new(
            MergePlan::new(vec![first, second]),
            vec![pdf_source(10, pdf_path, vec![PageSize::A4_PORTRAIT; 2])],
        );
        let engine = FakePdfEngine::new();

        Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap();

        let composed = engine.last_composed().unwrap();
        assert!(matches!(
            composed.entries[0],
            ComposeEntry::PdfPage { rotation, .. }
                if rotation == Rotation::from_quarter_turns(1)
        ));
        assert!(matches!(
            composed.entries[1],
            ComposeEntry::PdfPage { rotation, .. }
                if rotation == Rotation::from_quarter_turns(3)
        ));
    }

    #[test]
    fn an_image_slots_own_rotation_does_not_change_its_fit_to() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "letter.pdf");
        let image_path = create_file(&temp_dir, "image.jpg");
        let mut image_slot = slot(2, 20, 0);
        image_slot.rotation = Rotation::from_quarter_turns(1);
        let document = MergeDocument::new(
            MergePlan::new(vec![slot(1, 10, 0), image_slot]),
            vec![
                pdf_source(10, pdf_path, vec![letter()]),
                image_source(20, image_path),
            ],
        );
        let engine = FakePdfEngine::new();

        Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap();

        let composed = engine.last_composed().unwrap();
        match composed.entries[1] {
            ComposeEntry::Image {
                fit_to, rotation, ..
            } => {
                assert_eq!(fit_to.size_class(), letter().size_class());
                assert_eq!(rotation, Rotation::from_quarter_turns(1));
            }
            ref other => panic!("expected an image entry, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_source_aborts_before_the_engine_is_called() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, document) = plan_with_nonexistent_source();
        let err = Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap_err();
        assert!(matches!(err, ComposeError::SourceMissing { .. }));
        assert!(
            engine.last_composed().is_none(),
            "the engine must not be invoked"
        );
    }

    #[test]
    fn an_empty_plan_is_rejected() {
        let err = Compose {
            pdf: &FakePdfEngine::new(),
        }
        .execute(
            &MergeDocument::default(),
            Path::new("/out.pdf"),
            &NullProgress,
        )
        .unwrap_err();
        assert_eq!(err, ComposeError::EmptyPlan);
    }

    #[test]
    fn progress_is_reported_for_every_entry_and_ends_at_the_total() {
        let sink = RecordingProgress::default();
        let (_temp_dir, document) = plan_with_three_slots();
        Compose {
            pdf: &FakePdfEngine::new(),
        }
        .execute(&document, Path::new("/out.pdf"), &sink)
        .unwrap();
        let reports = sink.reports();
        assert_eq!(reports.last(), Some(&(3, 3)));
    }

    #[test]
    fn sources_in_a_failed_state_are_excluded_from_the_output() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let ready_path = create_file(&temp_dir, "ready.pdf");
        let missing_failed_path = temp_dir.path().join("encrypted.pdf");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, ready_path, vec![PageSize::A4_PORTRAIT]),
            SourceFile::failed(
                SourceId(20),
                missing_failed_path,
                SourceKind::Pdf,
                SourceStatus::Encrypted,
            ),
        ];
        let engine = FakePdfEngine::new();

        let document = MergeDocument::new(plan, sources);
        Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap();

        let composed = engine.last_composed().unwrap();
        assert_eq!(composed.entries.len(), 1);
        assert!(matches!(composed.entries[0], ComposeEntry::PdfPage { .. }));
    }

    #[test]
    fn a_document_with_only_unusable_sources_reports_the_real_situation() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let plan = MergePlan::new(vec![slot(1, 10, 0)]);
        let sources = vec![SourceFile::failed(
            SourceId(10),
            create_file(&temp_dir, "encrypted.pdf"),
            SourceKind::Pdf,
            SourceStatus::Encrypted,
        )];
        let document = MergeDocument::new(plan, sources);
        let engine = FakePdfEngine::new();

        let err = Compose { pdf: &engine }
            .execute(&document, Path::new("/out.pdf"), &NullProgress)
            .unwrap_err();

        assert_eq!(err, ComposeError::NoUsableSources);
        assert!(
            engine.last_composed().is_none(),
            "the engine must not be invoked"
        );
    }

    #[test]
    fn progress_reports_are_monotonic_and_called_exactly_total_times() {
        let sink = RecordingProgress::default();
        let (_temp_dir, document) = plan_with_three_slots();
        Compose {
            pdf: &FakePdfEngine::new(),
        }
        .execute(&document, Path::new("/out.pdf"), &sink)
        .unwrap();

        assert_eq!(sink.reports(), vec![(1, 3), (2, 3), (3, 3)]);
    }
}
