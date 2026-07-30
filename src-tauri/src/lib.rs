pub mod logging;
pub mod presentation;

use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use presentation::commands::{pdfium_health, PdfiumState};
use std::path::PathBuf;
use tauri::Manager;

fn load_pdfium(app: &tauri::App) -> Result<PdfiumEngine, String> {
    let mut candidates = Vec::new();
    let mut last_error = None;

    match app.path().resource_dir() {
        Ok(resource_dir) => candidates.push(resource_dir.join("resources/pdfium")),
        Err(error) => {
            last_error = Some(format!(
                "failed to resolve the PDFium resource directory: {error}"
            ));
        }
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("pdfium"),
    );

    for library_dir in candidates {
        match PdfiumEngine::new(&library_dir) {
            Ok(engine) => {
                tracing::info!(
                    path = %library_dir.display(),
                    version = %engine.version(),
                    "PDFium loaded"
                );
                return Ok(engine);
            }
            Err(error) => {
                tracing::warn!(
                    path = %library_dir.display(),
                    %error,
                    "failed to load PDFium"
                );
                last_error = Some(error.to_string());
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "PDFium engine is unavailable".to_owned()))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(error) = logging::init(app) {
                eprintln!("failed to initialize logging: {error}");
            } else {
                tracing::info!(
                    version = %env!("CARGO_PKG_VERSION"),
                    "pdf-tools starting"
                );
            }

            let pdfium = load_pdfium(app);
            if let Err(error) = &pdfium {
                tracing::error!(%error, "PDFium is unavailable");
            }
            app.manage(PdfiumState::new(pdfium));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![pdfium_health])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
