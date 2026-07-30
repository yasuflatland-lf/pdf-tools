use crate::application::errors::PdfError;
use pdfium_render::prelude::*;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

pub mod compose;
pub mod probe;

/// A thread-safe handle to the process-wide PDFium library.
pub struct PdfiumEngine {
    library: Mutex<Pdfium>,
    version: String,
}

impl PdfiumEngine {
    /// Loads PDFium from its platform-specific shared library in `library_dir`.
    pub fn new(library_dir: &Path) -> Result<Self, PdfError> {
        let library_path = Pdfium::pdfium_platform_library_name_at_path(library_dir);
        let bindings = Pdfium::bind_to_library(&library_path).map_err(|error| {
            PdfError::EngineUnavailable(format!(
                "failed to load the PDFium library at {}: {error}",
                library_path.display()
            ))
        })?;

        // PDFium exposes no runtime version call, so the bound FPDF API
        // version is the only version information available.
        let version = format!("FPDF API {:?}", bindings.version());

        Ok(Self {
            library: Mutex::new(Pdfium::new(bindings)),
            version,
        })
    }

    /// Returns the bound FPDF API version.
    pub fn version(&self) -> String {
        self.version.clone()
    }

    /// Runs `f` with exclusive access to the underlying Pdfium instance.
    ///
    /// PDFium keeps process-global state, so all calls are serialized through
    /// a mutex.
    pub fn with_library<T>(&self, f: impl FnOnce(&Pdfium) -> T) -> T {
        let library = self
            .library
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&library)
    }
}

impl fmt::Debug for PdfiumEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PdfiumEngine")
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}
