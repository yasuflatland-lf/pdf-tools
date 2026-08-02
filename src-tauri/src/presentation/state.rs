use std::sync::{Arc, Mutex, MutexGuard};

use pdf_tools_core::application::ports::{DirectoryWalker, ImageDecoder, PdfEngine};
use pdf_tools_core::application::session::PlanSession;
use pdf_tools_core::infrastructure::fs_walker::StdFsWalker;

/// The Tauri-managed application state.
///
/// Engines are held behind `Arc<dyn _>` so tests can drive the commands with
/// fakes instead of a real PDFium build. `PlanSession` owns the document and
/// its undo and redo history.
pub struct AppState {
    pub(crate) session: Mutex<PlanSession>,
    pub(crate) pdf: Arc<dyn PdfEngine>,
    pub(crate) images: Arc<dyn ImageDecoder>,
    pub(crate) walker: Arc<dyn DirectoryWalker>,
}

impl AppState {
    /// Creates an empty session backed by the given engines and the real
    /// filesystem. Tests that never expand a folder keep using this.
    pub fn with_engines(pdf: Arc<dyn PdfEngine>, images: Arc<dyn ImageDecoder>) -> Self {
        Self::with_ports(pdf, images, Arc::new(StdFsWalker))
    }

    /// Full injection, for the tests that drive folder expansion.
    pub fn with_ports(
        pdf: Arc<dyn PdfEngine>,
        images: Arc<dyn ImageDecoder>,
        walker: Arc<dyn DirectoryWalker>,
    ) -> Self {
        Self {
            session: Mutex::new(PlanSession::new()),
            pdf,
            images,
            walker,
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

    pub fn walker(&self) -> &dyn DirectoryWalker {
        self.walker.as_ref()
    }
}
