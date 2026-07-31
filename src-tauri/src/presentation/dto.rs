use pdf_tools_core::application::ports::MergeReport;
use pdf_tools_core::application::session::PlanSession;
use pdf_tools_core::domain::plan::PageSlot;
use pdf_tools_core::domain::source::{SourceFile, SourceKind, SourceStatus, UnreadableReason};
use serde::Serialize;
use ts_rs::TS;

/// The result of a successful merge.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MergeReportDto {
    pub page_count: u32,
    #[ts(type = "number")]
    pub bytes_written: u64,
}

/// Progress emitted while a merge is running.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ComposeProgressDto {
    pub done: u32,
    pub total: u32,
}

/// The whole document as the frontend sees it. Commands return a full snapshot
/// rather than a diff, so the UI never has to replay operations itself.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct PlanSnapshot {
    pub slots: Vec<PageSlotDto>,
    pub sources: Vec<SourceFileDto>,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// One page of one source at one position in the plan.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct PageSlotDto {
    // Identifiers are `u64` in Rust but `number` in TypeScript: the Tauri IPC
    // is JSON, so serde puts them on the wire as plain numbers, not `bigint`.
    #[ts(type = "number")]
    pub id: u64,
    #[ts(type = "number")]
    pub source: u64,
    pub page: u32,
}

/// A source file and the metadata the UI needs to label it. `file_name` is the
/// final path component, for display when the full path is too long.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SourceFileDto {
    #[ts(type = "number")]
    pub id: u64,
    pub path: String,
    pub file_name: String,
    pub kind: SourceKindDto,
    pub grouping: GroupingDto,
    pub page_count: u32,
    pub status: SourceStatusDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings/", rename_all = "lowercase")]
pub enum SourceKindDto {
    Pdf,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings/", rename_all = "lowercase")]
pub enum GroupingDto {
    Grouped,
    Ungrouped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings/", rename_all = "camelCase")]
pub enum UnreadableReasonDto {
    UnsupportedFormat,
    Damaged,
    Missing,
    EngineUnavailable,
}

/// Why a source does or does not contribute pages. Tagged so TypeScript can
/// narrow on `kind` and still reach `reason`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings/", rename_all = "camelCase")]
pub enum SourceStatusDto {
    Ready,
    Encrypted,
    Unreadable { reason: UnreadableReasonDto },
}

impl PlanSnapshot {
    /// Projects the application session onto the wire contract.
    pub fn from_session(session: &PlanSession) -> Self {
        Self {
            slots: session
                .plan()
                .slots()
                .iter()
                .map(PageSlotDto::from)
                .collect(),
            sources: session
                .sources()
                .iter()
                .map(|source| {
                    let grouping = if session.is_grouped(source.id) {
                        GroupingDto::Grouped
                    } else {
                        GroupingDto::Ungrouped
                    };
                    SourceFileDto::project(source, grouping)
                })
                .collect(),
            can_undo: session.can_undo(),
            can_redo: session.can_redo(),
        }
    }
}

impl From<&PageSlot> for PageSlotDto {
    fn from(slot: &PageSlot) -> Self {
        Self {
            id: slot.id.0,
            source: slot.source.0,
            page: slot.page.0,
        }
    }
}

impl SourceFileDto {
    fn project(source: &SourceFile, grouping: GroupingDto) -> Self {
        let path = source.path.to_string_lossy().into_owned();
        let file_name = source
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());

        Self {
            id: source.id.0,
            path,
            file_name,
            kind: match source.kind {
                SourceKind::Pdf => SourceKindDto::Pdf,
                SourceKind::Image => SourceKindDto::Image,
            },
            grouping,
            page_count: source.page_count,
            status: SourceStatusDto::from(&source.status),
        }
    }
}

impl From<&SourceStatus> for SourceStatusDto {
    fn from(status: &SourceStatus) -> Self {
        match status {
            SourceStatus::Ready => Self::Ready,
            SourceStatus::Encrypted => Self::Encrypted,
            SourceStatus::Unreadable(reason) => Self::Unreadable {
                reason: (*reason).into(),
            },
        }
    }
}

impl From<UnreadableReason> for UnreadableReasonDto {
    fn from(reason: UnreadableReason) -> Self {
        match reason {
            UnreadableReason::UnsupportedFormat => Self::UnsupportedFormat,
            UnreadableReason::Damaged => Self::Damaged,
            UnreadableReason::Missing => Self::Missing,
            UnreadableReason::EngineUnavailable => Self::EngineUnavailable,
        }
    }
}

impl From<MergeReport> for MergeReportDto {
    fn from(report: MergeReport) -> Self {
        Self {
            page_count: report.page_count,
            bytes_written: report.bytes_written,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pdf_tools_core::domain::ids::SourceId;

    use super::*;

    fn source(path: &str, kind: SourceKind, status: SourceStatus) -> SourceFile {
        SourceFile {
            id: SourceId(7),
            path: PathBuf::from(path),
            kind,
            page_count: 2,
            page_sizes: Vec::new(),
            status,
        }
    }

    #[test]
    fn a_source_is_projected_onto_the_wire_contract() {
        let dto = SourceFileDto::project(
            &source(
                "/deep/dir/invoice.pdf",
                SourceKind::Pdf,
                SourceStatus::Ready,
            ),
            GroupingDto::Ungrouped,
        );

        assert_eq!(dto.id, 7);
        assert_eq!(dto.path, "/deep/dir/invoice.pdf");
        assert_eq!(dto.file_name, "invoice.pdf");
        assert_eq!(dto.kind, SourceKindDto::Pdf);
        assert_eq!(dto.grouping, GroupingDto::Ungrouped);
        assert_eq!(dto.page_count, 2);
        assert_eq!(dto.status, SourceStatusDto::Ready);

        let value = serde_json::to_value(dto).unwrap();
        assert_eq!(value["kind"], serde_json::json!("pdf"));
        assert_eq!(value["grouping"], serde_json::json!("ungrouped"));
    }

    #[test]
    fn an_image_source_is_labelled_image() {
        let dto = SourceFileDto::project(
            &source("/p.png", SourceKind::Image, SourceStatus::Ready),
            GroupingDto::Grouped,
        );
        assert_eq!(dto.kind, SourceKindDto::Image);
        assert_eq!(dto.file_name, "p.png");
    }

    #[test]
    fn the_status_tag_matches_the_generated_typescript_literals() {
        // The generated SourceStatusDto.ts narrows on a lowercase `kind` tag,
        // so the serde output has to use exactly those literals.
        let ready = serde_json::to_value(SourceStatusDto::Ready).unwrap();
        assert_eq!(ready, serde_json::json!({ "kind": "ready" }));

        let encrypted = serde_json::to_value(SourceStatusDto::Encrypted).unwrap();
        assert_eq!(encrypted, serde_json::json!({ "kind": "encrypted" }));

        let unreadable = serde_json::to_value(SourceStatusDto::Unreadable {
            reason: UnreadableReasonDto::Damaged,
        })
        .unwrap();
        assert_eq!(
            unreadable,
            serde_json::json!({ "kind": "unreadable", "reason": "damaged" })
        );
    }

    #[test]
    fn slot_identifiers_cross_the_wire_as_plain_numbers() {
        let slot = PageSlot {
            id: pdf_tools_core::domain::ids::SlotId(3),
            source: SourceId(7),
            page: pdf_tools_core::domain::ids::PageIndex(1),
        };
        let value = serde_json::to_value(PageSlotDto::from(&slot)).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "id": 3, "source": 7, "page": 1 })
        );
    }

    #[test]
    fn compose_types_use_the_expected_wire_field_names() {
        let report = serde_json::to_value(MergeReportDto::from(MergeReport {
            page_count: 2,
            bytes_written: 4096,
        }))
        .unwrap();
        assert_eq!(
            report,
            serde_json::json!({ "page_count": 2, "bytes_written": 4096 })
        );

        let progress = serde_json::to_value(ComposeProgressDto { done: 1, total: 2 }).unwrap();
        assert_eq!(progress, serde_json::json!({ "done": 1, "total": 2 }));
    }
}
