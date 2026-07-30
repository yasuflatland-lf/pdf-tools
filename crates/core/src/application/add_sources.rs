use std::path::{Path, PathBuf};

use crate::application::errors::PdfError;
use crate::application::ports::{ImageDecoder, PdfEngine};
use crate::domain::ids::{IdSequence, PageIndex};
use crate::domain::operations::insert_at;
use crate::domain::plan::{MergePlan, PageSlot};
use crate::domain::source::{Grouping, SourceFile, SourceKind, SourceStatus};

pub struct AddSources<'a> {
    pub pdf: &'a dyn PdfEngine,
    pub images: &'a dyn ImageDecoder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddSourcesResult {
    pub plan: MergePlan,
    pub sources: Vec<SourceFile>,
}

impl AddSources<'_> {
    /// Appends the given files to the end of the plan. Never fails as a whole:
    /// unreadable or encrypted files are recorded with a failed status and
    /// contribute no slots, so one bad file cannot block the rest.
    pub fn execute(
        &self,
        plan: &MergePlan,
        sources: &[SourceFile],
        ids: &mut IdSequence,
        paths: &[PathBuf],
    ) -> AddSourcesResult {
        let mut result_sources = sources.to_vec();
        let mut new_slots = Vec::new();

        for path in paths {
            let Some(kind) = source_kind(path) else {
                continue;
            };
            let source_id = ids.next_source();

            let source = match kind {
                SourceKind::Pdf => match self.pdf.probe(path) {
                    Ok(info) if info.encrypted => SourceFile {
                        id: source_id,
                        path: path.clone(),
                        kind,
                        grouping: Grouping::Grouped,
                        page_count: 0,
                        page_sizes: Vec::new(),
                        status: SourceStatus::Encrypted,
                    },
                    Ok(info) => {
                        new_slots.extend((0..info.page_count).map(|page| PageSlot {
                            id: ids.next_slot(),
                            source: source_id,
                            page: PageIndex(page),
                        }));
                        SourceFile {
                            id: source_id,
                            path: path.clone(),
                            kind,
                            grouping: Grouping::Grouped,
                            page_count: info.page_count,
                            page_sizes: info.page_sizes,
                            status: SourceStatus::Ready,
                        }
                    }
                    Err(PdfError::Encrypted { .. }) => SourceFile {
                        id: source_id,
                        path: path.clone(),
                        kind,
                        grouping: Grouping::Grouped,
                        page_count: 0,
                        page_sizes: Vec::new(),
                        status: SourceStatus::Encrypted,
                    },
                    Err(error) => SourceFile {
                        id: source_id,
                        path: path.clone(),
                        kind,
                        grouping: Grouping::Grouped,
                        page_count: 0,
                        page_sizes: Vec::new(),
                        status: SourceStatus::Unreadable {
                            reason: error.to_string(),
                        },
                    },
                },
                SourceKind::Image => match self.images.probe(path) {
                    Ok(_) => {
                        new_slots.push(PageSlot {
                            id: ids.next_slot(),
                            source: source_id,
                            page: PageIndex(0),
                        });
                        SourceFile {
                            id: source_id,
                            path: path.clone(),
                            kind,
                            grouping: Grouping::Grouped,
                            page_count: 1,
                            page_sizes: Vec::new(),
                            status: SourceStatus::Ready,
                        }
                    }
                    Err(error) => SourceFile {
                        id: source_id,
                        path: path.clone(),
                        kind,
                        grouping: Grouping::Grouped,
                        page_count: 0,
                        page_sizes: Vec::new(),
                        status: SourceStatus::Unreadable {
                            reason: error.to_string(),
                        },
                    },
                },
            };

            result_sources.push(source);
        }

        AddSourcesResult {
            plan: insert_at(plan, plan.len(), &new_slots),
            sources: result_sources,
        }
    }
}

fn source_kind(path: &Path) -> Option<SourceKind> {
    let extension = path.extension()?.to_str()?;

    if extension.eq_ignore_ascii_case("pdf") {
        Some(SourceKind::Pdf)
    } else if ["jpg", "jpeg", "png", "gif"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        Some(SourceKind::Image)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::application::errors::{ImageError, PdfError};
    use crate::domain::geometry::PageSize;
    use crate::domain::ids::{IdSequence, PageIndex, SlotId, SourceId};
    use crate::domain::plan::{MergePlan, PageSlot};
    use crate::domain::source::{DocumentInfo, Grouping, ImageInfo, SourceKind, SourceStatus};
    use crate::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    use super::*;

    fn doc(pages: u32) -> DocumentInfo {
        DocumentInfo {
            page_count: pages,
            page_sizes: vec![PageSize::A4_PORTRAIT; pages as usize],
            encrypted: false,
        }
    }

    #[test]
    fn a_three_page_pdf_appends_three_slots() {
        let pdf = FakePdfEngine::new().with_document("/a.pdf", doc(3));
        let images = FakeImageDecoder::new();
        let mut ids = IdSequence::default();
        let result = AddSources {
            pdf: &pdf,
            images: &images,
        }
        .execute(&MergePlan::default(), &[], &mut ids, &["/a.pdf".into()]);
        assert_eq!(result.plan.len(), 3);
        assert_eq!(result.sources[0].page_count, 3);
        assert_eq!(result.sources[0].grouping, Grouping::Grouped);
        assert_eq!(result.sources[0].path, PathBuf::from("/a.pdf"));
        assert_eq!(result.sources[0].status, SourceStatus::Ready);
        assert_eq!(result.sources[0].page_sizes, vec![PageSize::A4_PORTRAIT; 3]);

        // Every slot must point back at the source it came from, in page order.
        let source_id = result.sources[0].id;
        assert!(result
            .plan
            .slots()
            .iter()
            .all(|slot| slot.source == source_id));
        assert_eq!(
            result
                .plan
                .slots()
                .iter()
                .map(|slot| slot.page)
                .collect::<Vec<_>>(),
            vec![PageIndex(0), PageIndex(1), PageIndex(2)]
        );
    }

    #[test]
    fn an_image_appends_exactly_one_slot() {
        let images = FakeImageDecoder::new().with_image(
            "/p.png",
            ImageInfo {
                width_px: 400,
                height_px: 300,
            },
        );
        let result = AddSources {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/p.png".into()],
        );
        assert_eq!(result.plan.len(), 1);
        assert_eq!(result.sources[0].kind, SourceKind::Image);
        assert_eq!(result.sources[0].page_count, 1);
        assert_eq!(result.sources[0].grouping, Grouping::Grouped);
        assert_eq!(result.sources[0].status, SourceStatus::Ready);
        // An image is laid out at the plan's dominant page size, so it carries
        // no page size of its own.
        assert!(result.sources[0].page_sizes.is_empty());
        assert_eq!(result.plan.slots()[0].source, result.sources[0].id);
        assert_eq!(result.plan.slots()[0].page, PageIndex(0));
    }

    #[test]
    fn an_encrypted_pdf_is_recorded_but_contributes_no_slots() {
        let pdf = FakePdfEngine::new().with_failure(
            "/locked.pdf",
            PdfError::Encrypted {
                path: "/locked.pdf".into(),
            },
        );
        let result = AddSources {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/locked.pdf".into()],
        );
        assert_eq!(result.plan.len(), 0);
        assert_eq!(result.sources[0].status, SourceStatus::Encrypted);
    }

    #[test]
    fn one_bad_file_does_not_block_the_others() {
        let pdf = FakePdfEngine::new()
            .with_document("/good.pdf", doc(2))
            .with_failure(
                "/bad.pdf",
                PdfError::Unreadable {
                    path: "/bad.pdf".into(),
                    reason: "broken xref".into(),
                },
            );
        let result = AddSources {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/bad.pdf".into(), "/good.pdf".into()],
        );
        assert_eq!(result.plan.len(), 2); // only the good file contributed
        assert_eq!(result.sources.len(), 2); // but both are recorded

        // Sources keep the input order, and only the good one owns slots.
        assert_eq!(result.sources[0].path, PathBuf::from("/bad.pdf"));
        assert!(matches!(
            result.sources[0].status,
            SourceStatus::Unreadable { .. }
        ));
        assert_eq!(result.sources[1].path, PathBuf::from("/good.pdf"));
        assert_eq!(result.sources[1].status, SourceStatus::Ready);
        let good_id = result.sources[1].id;
        assert!(result
            .plan
            .slots()
            .iter()
            .all(|slot| slot.source == good_id));
    }

    #[test]
    fn unsupported_extensions_are_skipped_entirely() {
        let result = AddSources {
            pdf: &FakePdfEngine::new(),
            images: &FakeImageDecoder::new(),
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/notes.txt".into()],
        );
        assert_eq!(result.sources.len(), 0);
    }

    #[test]
    fn new_slots_are_appended_after_existing_ones() {
        let existing = PageSlot {
            id: SlotId(99),
            source: SourceId(42),
            page: PageIndex(7),
        };
        let plan = MergePlan::new(vec![existing.clone()]);
        let pdf = FakePdfEngine::new().with_document("/new.pdf", doc(2));
        let result = AddSources {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(&plan, &[], &mut IdSequence::default(), &["/new.pdf".into()]);

        assert_eq!(result.plan.len(), 3);
        assert_eq!(result.plan.slots()[0], existing);
        assert_eq!(result.plan.slots()[1].page, PageIndex(0));
        assert_eq!(result.plan.slots()[2].page, PageIndex(1));
        assert_ne!(result.plan.slots()[1].id, result.plan.slots()[2].id);
        assert_ne!(result.plan.slots()[1].id, existing.id);
        assert_ne!(result.plan.slots()[2].id, existing.id);
    }

    #[test]
    fn pdf_extensions_are_matched_case_insensitively() {
        let pdf = FakePdfEngine::new()
            .with_document("/lower.pdf", doc(1))
            .with_document("/UPPER.PDF", doc(1));
        let result = AddSources {
            pdf: &pdf,
            images: &FakeImageDecoder::new(),
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/lower.pdf".into(), "/UPPER.PDF".into()],
        );

        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.plan.len(), 2);
        assert!(result
            .sources
            .iter()
            .all(|source| source.kind == SourceKind::Pdf));
        // Each recorded source draws its own id from the sequence.
        assert_ne!(result.sources[0].id, result.sources[1].id);
    }

    #[test]
    fn a_failed_image_is_recorded_without_a_slot() {
        let images = FakeImageDecoder::new().with_failure(
            "/broken.gif",
            ImageError::Unreadable {
                path: "/broken.gif".into(),
                reason: "truncated data".into(),
            },
        );
        let result = AddSources {
            pdf: &FakePdfEngine::new(),
            images: &images,
        }
        .execute(
            &MergePlan::default(),
            &[],
            &mut IdSequence::default(),
            &["/broken.gif".into()],
        );

        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.plan.len(), 0);
        assert!(matches!(
            result.sources[0].status,
            SourceStatus::Unreadable { .. }
        ));
    }
}
