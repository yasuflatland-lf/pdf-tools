use pdf_tools_core::domain::plan::{MergePlan, PageSlot};
use pdf_tools_core::domain::source::{Grouping, SourceFile, SourceKind, SourceStatus};
use serde::Serialize;
use ts_rs::TS;

/// The whole document as the frontend sees it. Commands return a full snapshot
/// rather than a diff, so the UI never has to replay operations itself.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct PlanSnapshot {
    pub slots: Vec<PageSlotDto>,
    pub sources: Vec<SourceFileDto>,
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

/// A source file and the metadata the UI needs to label it. `kind` is `"pdf"`
/// or `"image"`; `grouping` is `"grouped"` or `"ungrouped"`; `file_name` is the
/// final path component, for display when the full path is too long.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SourceFileDto {
    #[ts(type = "number")]
    pub id: u64,
    pub path: String,
    pub file_name: String,
    pub kind: String,
    pub grouping: String,
    pub page_count: u32,
    pub status: SourceStatusDto,
}

/// Why a source does or does not contribute pages. Tagged so TypeScript can
/// narrow on `kind` and still reach `reason`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings/", rename_all = "camelCase")]
pub enum SourceStatusDto {
    Ready,
    Encrypted,
    Unreadable { reason: String },
}

impl PlanSnapshot {
    /// Projects the domain document onto the wire contract.
    pub fn from_document(plan: &MergePlan, sources: &[SourceFile]) -> Self {
        Self {
            slots: plan.slots().iter().map(PageSlotDto::from).collect(),
            sources: sources.iter().map(SourceFileDto::from).collect(),
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

impl From<&SourceFile> for SourceFileDto {
    fn from(source: &SourceFile) -> Self {
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
                SourceKind::Pdf => "pdf",
                SourceKind::Image => "image",
            }
            .into(),
            grouping: match source.grouping {
                Grouping::Grouped => "grouped",
                Grouping::Ungrouped => "ungrouped",
            }
            .into(),
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
            SourceStatus::Unreadable { reason } => Self::Unreadable {
                reason: reason.clone(),
            },
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
            grouping: Grouping::Ungrouped,
            page_count: 2,
            page_sizes: Vec::new(),
            status,
        }
    }

    #[test]
    fn a_source_is_projected_onto_the_wire_contract() {
        let dto = SourceFileDto::from(&source(
            "/deep/dir/invoice.pdf",
            SourceKind::Pdf,
            SourceStatus::Ready,
        ));

        assert_eq!(dto.id, 7);
        assert_eq!(dto.path, "/deep/dir/invoice.pdf");
        assert_eq!(dto.file_name, "invoice.pdf");
        assert_eq!(dto.kind, "pdf");
        assert_eq!(dto.grouping, "ungrouped");
        assert_eq!(dto.page_count, 2);
        assert_eq!(dto.status, SourceStatusDto::Ready);
    }

    #[test]
    fn an_image_source_is_labelled_image() {
        let dto = SourceFileDto::from(&source("/p.png", SourceKind::Image, SourceStatus::Ready));
        assert_eq!(dto.kind, "image");
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
            reason: "broken xref".to_owned(),
        })
        .unwrap();
        assert_eq!(
            unreadable,
            serde_json::json!({ "kind": "unreadable", "reason": "broken xref" })
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
}
