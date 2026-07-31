use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::{ComposePlan, ImageDecoder, MergeReport, PdfEngine};
use pdf_tools_core::application::session::PlanSession;
use pdf_tools_core::domain::geometry::{RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use pdf_tools_core::domain::source::DocumentInfo;

/// The Tauri-managed application state.
///
/// Engines are held behind `Arc<dyn _>` so tests can drive the commands with
/// fakes instead of a real PDFium build. `PlanSession` owns the document and
/// its undo and redo history.
pub struct AppState {
    pub(crate) session: Mutex<PlanSession>,
    pub(crate) pdf: Arc<dyn PdfEngine>,
    pub(crate) images: Arc<dyn ImageDecoder>,
}

impl AppState {
    /// Creates an empty session backed by the given engines.
    pub fn with_engines(pdf: Arc<dyn PdfEngine>, images: Arc<dyn ImageDecoder>) -> Self {
        Self {
            session: Mutex::new(PlanSession::new()),
            pdf,
            images,
        }
    }

    /// Returns exclusive access to the plan session.
    ///
    /// The fields are crate-private, so integration tests -- compiled as
    /// separate crates -- reach the state through these accessors.
    ///
    /// A poisoned lock is recovered from rather than propagated. A panicking
    /// command cannot leave the session inconsistent: every mutation replaces
    /// whole values between `begin_change` and `finish_change`, so the worst a
    /// panic leaves behind is one undo entry that restores the same state.
    /// Propagating instead would fail every later command permanently, and the
    /// only recovery would be restarting the app, which discards the document
    /// the user has assembled.
    pub fn session(&self) -> MutexGuard<'_, PlanSession> {
        self.session.lock().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "the plan session lock was poisoned by a panicking command; \
                 recovering the document as it stood before the panic"
            );
            poisoned.into_inner()
        })
    }

    pub fn pdf(&self) -> &dyn PdfEngine {
        self.pdf.as_ref()
    }

    pub fn images(&self) -> &dyn ImageDecoder {
        self.images.as_ref()
    }
}

/// Stands in for the PDF engine when the PDFium library failed to load, so the
/// app still starts and reports the reason on every attempted operation.
pub struct UnavailablePdfEngine {
    reason: String,
}

impl UnavailablePdfEngine {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }

    fn error(&self) -> PdfError {
        PdfError::EngineUnavailable(self.reason.clone())
    }
}

impl PdfEngine for UnavailablePdfEngine {
    fn probe(&self, _src: &Path) -> Result<DocumentInfo, PdfError> {
        Err(self.error())
    }

    fn rasterize(
        &self,
        _src: &Path,
        _page: PageIndex,
        _spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        Err(self.error())
    }

    fn compose(&self, _plan: &ComposePlan, _dest: &Path) -> Result<MergeReport, PdfError> {
        Err(self.error())
    }
}
