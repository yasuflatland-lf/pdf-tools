use std::path::{Path, PathBuf};

use crate::application::errors::PdfError;
use crate::application::ports::{ComposeEntry, ComposePlan, MergeReport, PdfEngine};
use crate::domain::page_size::dominant_page_size;
use crate::domain::plan::MergePlan;
use crate::domain::source::{SourceFile, SourceKind, SourceStatus};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ComposeError {
    #[error("the source file is no longer present: {path}")]
    SourceMissing { path: PathBuf },
    #[error("nothing to merge: the plan is empty")]
    EmptyPlan,
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
    /// Slots whose source is unknown or in a failed state contribute no entry:
    /// such a source never produces slots in practice, so the resolution step
    /// skips it defensively instead of failing. Every remaining source is
    /// checked before the engine runs, so a missing file can never leave a
    /// half-written output behind.
    pub fn execute(
        &self,
        plan: &MergePlan,
        sources: &[SourceFile],
        dest: &Path,
        progress: &dyn ProgressSink,
    ) -> Result<MergeReport, ComposeError> {
        if plan.is_empty() {
            return Err(ComposeError::EmptyPlan);
        }

        let fit_to = dominant_page_size(plan, sources);
        let entries = plan
            .slots()
            .iter()
            .filter_map(|slot| {
                let source = sources.iter().find(|source| source.id == slot.source)?;
                if source.status != SourceStatus::Ready {
                    return None;
                }

                Some(match source.kind {
                    SourceKind::Pdf => ComposeEntry::PdfPage {
                        path: source.path.clone(),
                        page: slot.page,
                    },
                    SourceKind::Image => ComposeEntry::Image {
                        path: source.path.clone(),
                        fit_to,
                    },
                })
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return Err(ComposeError::EmptyPlan);
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
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::{PageIndex, SlotId, SourceId};
    use crate::domain::plan::PageSlot;
    use crate::domain::source::{Grouping, SourceKind, SourceStatus};
    use crate::infrastructure::fake_engine::FakePdfEngine;

    const LETTER: PageSize = PageSize {
        width_pt: 612.0,
        height_pt: 792.0,
    };

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
        }
    }

    fn pdf_source(id: u64, path: PathBuf, page_sizes: Vec<PageSize>) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path,
            kind: SourceKind::Pdf,
            grouping: Grouping::Grouped,
            page_count: page_sizes.len() as u32,
            page_sizes,
            status: SourceStatus::Ready,
        }
    }

    fn image_source(id: u64, path: PathBuf) -> SourceFile {
        SourceFile {
            id: SourceId(id),
            path,
            kind: SourceKind::Image,
            grouping: Grouping::Ungrouped,
            page_count: 1,
            page_sizes: Vec::new(),
            status: SourceStatus::Ready,
        }
    }

    fn plan_with_pdf_and_image() -> (TempDir, MergePlan, Vec<SourceFile>) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "document.pdf");
        let image_path = create_file(&temp_dir, "image.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, pdf_path, vec![PageSize::A4_PORTRAIT]),
            image_source(20, image_path),
        ];
        (temp_dir, plan, sources)
    }

    fn plan_with_letter_pdf_and_image() -> (TempDir, MergePlan, Vec<SourceFile>) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let pdf_path = create_file(&temp_dir, "letter.pdf");
        let image_path = create_file(&temp_dir, "image.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, pdf_path, vec![LETTER]),
            image_source(20, image_path),
        ];
        (temp_dir, plan, sources)
    }

    fn plan_with_nonexistent_source() -> (TempDir, MergePlan, Vec<SourceFile>) {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let existing_path = create_file(&temp_dir, "existing.pdf");
        let missing_path = temp_dir.path().join("missing.png");
        let plan = MergePlan::new(vec![slot(1, 10, 0), slot(2, 20, 0)]);
        let sources = vec![
            pdf_source(10, existing_path, vec![PageSize::A4_PORTRAIT]),
            image_source(20, missing_path),
        ];
        (temp_dir, plan, sources)
    }

    fn plan_with_three_slots() -> (TempDir, MergePlan, Vec<SourceFile>) {
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
        (temp_dir, plan, sources)
    }

    #[test]
    fn the_resolved_plan_follows_the_slot_order() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, plan, sources) = plan_with_pdf_and_image();
        Compose { pdf: &engine }
            .execute(&plan, &sources, Path::new("/out.pdf"), &NullProgress)
            .unwrap();
        let composed = engine.last_composed().unwrap();
        assert!(matches!(composed.entries[0], ComposeEntry::PdfPage { .. }));
        assert!(matches!(composed.entries[1], ComposeEntry::Image { .. }));
    }

    #[test]
    fn every_image_is_fitted_to_the_dominant_page_size() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, plan, sources) = plan_with_letter_pdf_and_image();
        Compose { pdf: &engine }
            .execute(&plan, &sources, Path::new("/out.pdf"), &NullProgress)
            .unwrap();
        let composed = engine.last_composed().unwrap();
        match &composed.entries[1] {
            ComposeEntry::Image { fit_to, .. } => assert!(fit_to.approx_eq(&LETTER)),
            other => panic!("expected an image entry, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_source_aborts_before_the_engine_is_called() {
        let engine = FakePdfEngine::new();
        let (_temp_dir, plan, sources) = plan_with_nonexistent_source();
        let err = Compose { pdf: &engine }
            .execute(&plan, &sources, Path::new("/out.pdf"), &NullProgress)
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
            &MergePlan::default(),
            &[],
            Path::new("/out.pdf"),
            &NullProgress,
        )
        .unwrap_err();
        assert_eq!(err, ComposeError::EmptyPlan);
    }

    #[test]
    fn progress_is_reported_for_every_entry_and_ends_at_the_total() {
        let sink = RecordingProgress::default();
        let (_temp_dir, plan, sources) = plan_with_three_slots();
        Compose {
            pdf: &FakePdfEngine::new(),
        }
        .execute(&plan, &sources, Path::new("/out.pdf"), &sink)
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
            SourceFile {
                status: SourceStatus::Encrypted,
                ..pdf_source(20, missing_failed_path, vec![LETTER])
            },
        ];
        let engine = FakePdfEngine::new();

        Compose { pdf: &engine }
            .execute(&plan, &sources, Path::new("/out.pdf"), &NullProgress)
            .unwrap();

        let composed = engine.last_composed().unwrap();
        assert_eq!(composed.entries.len(), 1);
        assert!(matches!(composed.entries[0], ComposeEntry::PdfPage { .. }));
    }

    #[test]
    fn progress_reports_are_monotonic_and_called_exactly_total_times() {
        let sink = RecordingProgress::default();
        let (_temp_dir, plan, sources) = plan_with_three_slots();
        Compose {
            pdf: &FakePdfEngine::new(),
        }
        .execute(&plan, &sources, Path::new("/out.pdf"), &sink)
        .unwrap();

        assert_eq!(sink.reports(), vec![(1, 3), (2, 3), (3, 3)]);
    }
}
